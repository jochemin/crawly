use anyhow::Result;
use axum::extract::{Path, Query};
use axum::{routing::get, Json, Router};
use flate2::read::GzDecoder;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::fs::File;
use std::net::IpAddr;
use std::sync::Arc;
use tar::Archive;
use tokio::sync::broadcast;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[path = "common/common.rs"]
pub mod common;
#[path = "database/db.rs"]
pub mod db;
#[path = "p2p/p2p.rs"]
pub mod p2p;
use crate::db::NodeToScan;

#[derive(Serialize)]
struct Stats {
    total_nodes: i64,
    incoming_nodes: i64,
    archive_nodes: i64,
    ipv4_nodes: i64,
    ipv6_nodes: i64,
    tor_nodes: i64,
    core30_nodes: i64,
    knots_nodes: i64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();
    let database_url =
        env::var("DATABASE_URL").expect("La variable de entorno DATABASE_URL debe estar definida");

    let (shutdown_tx, _) = broadcast::channel(1);

    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&database_url)
        .await?;

    let semaphore = Arc::new(Semaphore::new(100));

    let db = Arc::new(crate::db::Database(pool));

    tracing::info!("Ejecutando migraciones de la base de datos...");
    sqlx::migrate!().run(&db.0).await?;
    tracing::info!("Migraciones completadas.");

    match db.get_total_nodes_count().await {
        Ok(count) => {
            if count == 0 {
                tracing::info!("Base de datos vacía. Añadiendo nodos semilla...");
                let initial_nodes = crate::common::first_nodes();
                tracing::info!("Insertando {} nodos semilla...", initial_nodes.len());

                for node in initial_nodes {
                    let addr_type = if node.is_ipv4() { "ipv4" } else { "ipv6" };
                    let address = node.ip().to_string();
                    let port = node.port();

                    if let Err(e) = db.upsert_addrv2_node(addr_type, &address, port, "").await {
                        tracing::warn!("Fallo al insertar nodo semilla {}: {}", address, e);
                    }
                }
                tracing::info!("Nodos semilla insertados.");
            } else {
                tracing::info!("La base de datos ya existe y contiene {} nodos.", count);
            }
        }
        Err(e) => {
            anyhow::bail!("Fallo crítico al contar nodos en la BBDD: {}", e);
        }
    }

    let app_state = db.clone();

    let app = Router::new()
        .route("/api/stats", get(get_stats))
        .route("/api/nodes", get(get_recent_nodes_api))
        .route("/api/node/{address}", get(find_node_api))
        .route("/api/software_stats", get(get_software_stats))
        .route("/api/incoming_stats", get(get_incoming_stats_api))
        .route("/api/stats/history", get(get_historical_stats))
        .route("/api/nodes/search", get(search_nodes_api))
        .layer(CorsLayer::permissive())
        .layer(axum::Extension(app_state))
        .fallback_service(ServeDir::new("public"));

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Servidor web escuchando en {}", addr);

    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Fallo en el servidor web: {}", e);
        }
    });

    let db_clone_listener = db.clone();
    let shutdown_rx_listener = shutdown_tx.subscribe();
    tokio::spawn(async move {
        run_listener_task(
            crate::db::Database(db_clone_listener.0.clone()),
            shutdown_rx_listener,
        )
        .await;
    });

    let db_clone_crawler = db.clone();
    let shutdown_rx_crawler = shutdown_tx.subscribe();
    let semaphore_clone = semaphore.clone();
    tokio::spawn(async move {
        run_crawler_task(
            crate::db::Database(db_clone_crawler.0.clone()),
            semaphore_clone,
            shutdown_rx_crawler,
        )
        .await;
    });

    let geo_ip_reader = common::GeoIpReader::new()?;
    let asn_reader = common::GeoIpAsnReader::new()?;

    let sched = JobScheduler::new().await?;
    let db_clone_snapshot = db.clone();

    sched
        .add(Job::new_async("0 0 * * * *", move |_uuid, _l| {
            let db = db_clone_snapshot.clone();
            Box::pin(async move {
                if let Err(e) = take_hourly_snapshot(crate::db::Database(db.0.clone())).await {
                    tracing::error!("Fallo al tomar snapshot horario: {}", e);
                }
            })
        })?)
        .await?;

    sched.start().await?;

    tokio::spawn(run_db_cleanup_task((*db).clone(), shutdown_tx.subscribe()));
    tokio::spawn(run_ip_enrichment_task(
        (*db).clone(),
        geo_ip_reader.clone(),
        asn_reader.clone(),
    ));
    tokio::spawn(run_geoip_update_task(
        geo_ip_reader.clone(),
        asn_reader.clone(),
    ));

    match tokio::signal::ctrl_c().await {
        Ok(()) => tracing::info!("Recibida señal de terminación (Ctrl+C)"),
        Err(err) => tracing::error!("Error escuchando señal de terminación: {}", err),
    }

    let _ = shutdown_tx.send(());

    tokio::time::sleep(Duration::from_secs(2)).await;
    tracing::info!("Apagado completado.");

    Ok(())
}

async fn run_db_cleanup_task(db: db::Database, mut shutdown_rx: broadcast::Receiver<()>) {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                tracing::info!("[Mantenimiento] Ejecutando limpieza de la base de datos...");
                if let Err(e) = db.clean_db().await {
                    tracing::error!("[Mantenimiento] Fallo al limpiar la base de datos: {}", e);
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("[Mantenimiento] Tarea de limpieza terminando...");
                break;
            }
        }
    }
}

async fn run_ip_enrichment_task(
    db: db::Database,
    city_reader: common::GeoIpReader,
    asn_reader: common::GeoIpAsnReader,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(300));

    loop {
        interval.tick().await;
        if let Ok(ips) = db.ip_info_list().await {
            if ips.is_empty() {
                continue;
            }

            tracing::info!(
                "[Mantenimiento] Enriqueciendo {} direcciones IP.",
                ips.len()
            );
            for ip_str in ips {
                if let Ok(ip_addr) = ip_str.parse::<IpAddr>() {
                    let geo_info = city_reader.lookup(ip_addr).unwrap_or_default();
                    let asn_info = asn_reader.lookup(ip_addr).unwrap_or_default();
                    let isp = &asn_info.isp;
                    let asn_str = asn_info.asn.to_string();

                    if let Err(e) = db
                        .update_ip_info(
                            &ip_str,
                            &geo_info.country,
                            &geo_info.city,
                            geo_info.latitude,
                            geo_info.longitude,
                            isp,
                            &asn_str,
                        )
                        .await
                    {
                        tracing::error!(
                            "[Mantenimiento] Fallo al actualizar info de IP {}: {}",
                            ip_str,
                            e
                        );
                    }
                }
            }
        }
    }
}

async fn run_geoip_update_task(
    city_reader: common::GeoIpReader,
    asn_reader: common::GeoIpAsnReader,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60 * 24 * 7));
    let client = reqwest::Client::new();
    let license_key = match std::env::var("MAXMIND_LICENSE_KEY") {
        Ok(key) => key,
        Err(_) => {
            tracing::error!("[Mantenimiento] MAXMIND_LICENSE_KEY no encontrada. La actualización automática de GeoIP está desactivada.");
            return;
        }
    };

    loop {
        interval.tick().await;
        tracing::info!("[Mantenimiento] Buscando actualizaciones para bases de datos GeoIP...");

        if let Err(e) = download_and_update_maxmind_db(
            &client,
            &license_key,
            "GeoLite2-City",
            "GeoLite2-City.mmdb",
        )
        .await
        {
            tracing::error!("[Mantenimiento] Fallo al actualizar GeoLite2-City: {}", e);
        } else if let Err(e) = city_reader.reload() {
            tracing::error!("[Mantenimiento] Fallo al recargar GeoLite2-City: {}", e);
        } else {
            tracing::info!("[Mantenimiento] GeoLite2-City actualizada y recargada.");
        }

        sleep(Duration::from_secs(5)).await;

        if let Err(e) = download_and_update_maxmind_db(
            &client,
            &license_key,
            "GeoLite2-ASN",
            "GeoLite2-ASN.mmdb",
        )
        .await
        {
            tracing::error!("[Mantenimiento] Fallo al actualizar GeoLite2-ASN: {}", e);
        } else if let Err(e) = asn_reader.reload() {
            tracing::error!("[Mantenimiento] Fallo al recargar GeoLite2-ASN: {}", e);
        } else {
            tracing::info!("[Mantenimiento] GeoLite2-ASN actualizada y recargada.");
        }
    }
}

async fn download_and_update_maxmind_db(
    client: &reqwest::Client,
    license_key: &str,
    edition_id: &str,
    final_filename: &str,
) -> Result<()> {
    tracing::info!("[Mantenimiento] Descargando {}...", edition_id);

    let download_url = format!(
        "https://download.maxmind.com/app/geoip_download?edition_id={}&license_key={}&suffix=tar.gz",
        edition_id, license_key
    );

    let response = client.get(&download_url).send().await?;
    let content = response.bytes().await?;
    let temp_tar_path = std::env::temp_dir().join(format!("{}.tar.gz", edition_id));
    let mut temp_tar_file = tokio::fs::File::create(&temp_tar_path).await?;
    tokio::io::copy(&mut content.as_ref(), &mut temp_tar_file).await?;

    let temp_extract_path = std::env::temp_dir().join(format!("{}_extract", edition_id));
    let tar_path_clone = temp_tar_path.clone();
    let extract_path_for_task = temp_extract_path.clone();

    tokio::task::spawn_blocking(move || -> Result<(), std::io::Error> {
        let tar_gz = File::open(tar_path_clone)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        std::fs::create_dir_all(&extract_path_for_task)?;
        archive.unpack(&extract_path_for_task)?;
        Ok(())
    })
    .await??;

    if let Some(entry) = std::fs::read_dir(&temp_extract_path)?
        .flatten()
        .find(|e| e.path().extension().map_or(false, |ext| ext == "mmdb"))
    {
        let temp_mmdb_path = entry.path();

        let mut final_path = std::env::current_dir()?;
        final_path.push("database");
        final_path.push(final_filename);

        if let Some(parent) = final_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        tracing::info!(
            "[Mantenimiento] Guardando {} en {}",
            final_filename,
            final_path.display()
        );
        tokio::fs::rename(temp_mmdb_path, final_path).await?;
    }

    let _ = tokio::fs::remove_file(&temp_tar_path).await;
    let _ = tokio::fs::remove_dir_all(&temp_extract_path).await;

    Ok(())
}

async fn get_stats(
    axum::Extension(db): axum::Extension<Arc<db::Database>>,
) -> impl axum::response::IntoResponse {
    let (total, incoming, archive, ipv4, ipv6, tor, core30, knots) = tokio::join!(
        db.get_total_nodes_count(),
        db.get_incoming_nodes_count(),
        db.get_archive_nodes_count(),
        db.get_ipv4_nodes_count(),
        db.get_ipv6_nodes_count(),
        db.get_tor_nodes_count(),
        db.get_core30_nodes_count(),
        db.get_knots_nodes_count()
    );

    let stats = Stats {
        total_nodes: total.unwrap_or(0),
        incoming_nodes: incoming.unwrap_or(0),
        archive_nodes: archive.unwrap_or(0),
        ipv4_nodes: ipv4.unwrap_or(0),
        ipv6_nodes: ipv6.unwrap_or(0),
        tor_nodes: tor.unwrap_or(0),
        core30_nodes: core30.unwrap_or(0),
        knots_nodes: knots.unwrap_or(0),
    };

    Json(stats)
}

async fn find_node_api(
    axum::Extension(db): axum::Extension<Arc<db::Database>>,
    Path(address): Path<String>,
) -> impl axum::response::IntoResponse {
    tracing::debug!(
        "Buscando nodo. IP recibida de la URL: '{}', longitud: {}",
        address,
        address.len()
    );

    match db.find_node_by_address(&address).await {
        Ok(Some(node)) => Ok(Json(node)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_software_stats(
    axum::Extension(db): axum::Extension<Arc<db::Database>>,
) -> impl axum::response::IntoResponse {
    match db.get_software_version_stats().await {
        Ok(stats) => (axum::http::StatusCode::OK, Json(stats)),
        Err(e) => {
            tracing::error!("Fallo al obtener stats de software: {:?}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(vec![]))
        }
    }
}

async fn run_listener_task(db: db::Database, mut shutdown_rx: broadcast::Receiver<()>) {
    let listener = match tokio::net::TcpListener::bind("0.0.0.0:8333").await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("[Listener] Fallo al iniciar listener en puerto 8333: {}. La escucha pasiva está desactivada.", e);
            return;
        }
    };
    tracing::info!("[Listener] Escuchando conexiones Bitcoin entrantes en el puerto 8333...");

    loop {
        tokio::select! {
            Ok((socket, addr)) = listener.accept() => {
                tracing::info!("[Listener] Nueva conexión entrante aceptada desde: {}", addr);
                let db_clone = db.clone();

                tokio::spawn(async move {
                    if let Err(e) = p2p::handle_connection(&db_clone, socket, addr).await {
                         tracing::warn!("[Listener] Conversación entrante con {} falló: {}", addr, e);
                    }
                });
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("[Listener] Señal de apagado recibida, dejando de aceptar conexiones.");
                break;
            }
        }
    }
}

async fn run_crawler_task(
    db: db::Database,
    semaphore: Arc<Semaphore>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    tracing::info!("[Crawler] Tarea de sondeo activo iniciada.");
    let mut interval = tokio::time::interval(Duration::from_secs(10));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match db.get_nodes_to_scan(100).await {
                    Ok(nodes) if !nodes.is_empty() => {
                        let num_nodes = nodes.len();
                        tracing::info!("[Crawler] Lanzando análisis para {} nodos en paralelo.", num_nodes);

                        let mut tasks = Vec::with_capacity(num_nodes);

                        for node in nodes {
                            let db_clone = db.clone();
                            let sem_clone = semaphore.clone();

                            let task_handle = tokio::spawn(async move {
                                let _permit = sem_clone.acquire_owned().await.unwrap();

                                let connection_result = match node {
                                    NodeToScan::Ip(socket_addr) => {
                                        let addr_str = socket_addr.to_string();
                                        match p2p::converse(&db_clone, socket_addr).await {
                                            Ok(_) => Ok(addr_str),
                                            Err(e) => {
                                                tracing::debug!("[Task] La conexión con {} falló: {}", addr_str, e);
                                                Err((addr_str, e))
                                            }
                                        }
                                    },
                                    NodeToScan::Tor { address, port } => {
                                        let full_address = format!("{}:{}", address, port);
                                        match p2p::converse_tor(&db_clone, &address, port).await {
                                            Ok(_) => Ok(full_address),
                                            Err(e) => {
                                                tracing::debug!("[Task] La conexión Tor con {} falló: {}", full_address, e);
                                                Err((full_address, e))
                                            }
                                        }
                                    }
                                    NodeToScan::I2p { address, port } => {
                                        let full_address = format!("{}:{}", address, port);
                                        match p2p::converse_i2p(&db_clone, &address, port).await {
                                            Ok(_) => Ok(full_address),
                                            Err(e) => {
                                                tracing::debug!("[Task] La conexión I2P con {} falló: {}", full_address, e);
                                                Err((full_address, e))
                                            }
                                        }
                                    }
                                };

                                connection_result
                            });

                            tasks.push(task_handle);
                        }

                        tracing::debug!("[Crawler] Esperando que terminen {} tareas...", tasks.len());
                        let results = join_all(tasks).await;
                        tracing::info!("[Crawler] Lote de {} nodos completado.", num_nodes);

                        for result in results {
                            match result {
                                Ok(Ok(_addr_str)) => {
                                }
                                Ok(Err((addr_str, _error))) => {
                                    if let Err(db_err) = db.handle_failed_connection(&addr_str).await {
                                        tracing::error!("[Crawler DB] Fallo de BBDD (failure) {}: {}", addr_str, db_err);
                                    }
                                }
                                Err(join_err) => {
                                    tracing::error!("[Crawler Task] Fallo al ejecutar la tarea de conexión: {}", join_err);
                                }
                            }
                        }
                    }
                    Ok(_) => {
                        tracing::info!("[Crawler] No hay nodos disponibles. Esperando próximo ciclo.");
                    }
                    Err(e) => {
                        tracing::error!("[Crawler] Fallo al obtener nodos de la BBDD: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("[Crawler] Señal de apagado recibida, terminando...");
                break;
            }
        }
    }
}

async fn take_hourly_snapshot(db: db::Database) -> Result<()> {
    let now = chrono::Utc::now();

    let (total_res, incoming_res, archive_res, ipv4_res, ipv6_res, onion_res, top_software_res): (
        _,
        _,
        _,
        _,
        _,
        _,
        Result<Vec<db::SoftwareCount>, _>,
    ) = tokio::join!(
        db.get_total_nodes_count(),
        db.get_incoming_nodes_count(),
        db.get_archive_nodes_count(),
        db.get_ipv4_nodes_count(),
        db.get_ipv6_nodes_count(),
        db.get_tor_nodes_count(),
        db.get_top_software_stats()
    );

    let top_software_json = match top_software_res {
        Ok(stats) => json!(stats),
        Err(e) => {
            tracing::error!(
                "[Snapshot] Fallo al obtener top software stats: {:?}. Usando null.",
                e
            );
            serde_json::Value::Null
        }
    };

    sqlx::query!(
        r#"
        INSERT INTO hourly_stats (
            snapshot_time, total_nodes, incoming_nodes, archive_nodes,
            ipv4_nodes, ipv6_nodes, onion_nodes,
            top_software 
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8) 
        ON CONFLICT (snapshot_time) DO NOTHING
        "#,
        now,
        total_res.unwrap_or(0),
        incoming_res.unwrap_or(0),
        archive_res.unwrap_or(0),
        ipv4_res.unwrap_or(0),
        ipv6_res.unwrap_or(0),
        onion_res.unwrap_or(0),
        top_software_json
    )
    .execute(&db.0)
    .await?;

    tracing::info!(
        "[Snapshot] Instantánea horaria guardada correctamente (incluyendo top 10 software)."
    );
    Ok(())
}

#[derive(Deserialize)]
struct PaginationParams {
    page: Option<i64>,
    limit: Option<i64>,
}

async fn get_recent_nodes_api(
    Query(params): Query<PaginationParams>,
    axum::Extension(db): axum::Extension<Arc<db::Database>>,
) -> Json<Vec<db::NodeInfo>> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let nodes = db.get_recent_nodes(limit, offset).await.unwrap_or_default();
    Json(nodes)
}

#[derive(Deserialize)]
struct SearchParams {
    q: Option<String>,
}

async fn search_nodes_api(
    Query(params): Query<SearchParams>,
    axum::Extension(db): axum::Extension<Arc<db::Database>>,
) -> Json<Vec<db::NodeInfo>> {
    let query = params.q.unwrap_or_default();
    if query.trim().is_empty() {
        return Json(vec![]);
    }
    let nodes = db.search_nodes(&query).await.unwrap_or_default();
    Json(nodes)
}

async fn get_incoming_stats_api(
    axum::Extension(db): axum::Extension<Arc<db::Database>>,
) -> impl axum::response::IntoResponse {
    match db.get_incoming_stats_by_protocol().await {
        Ok(stats) => (axum::http::StatusCode::OK, Json(stats)),
        Err(e) => {
            tracing::error!("Fallo al obtener incoming_stats: {:?}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(vec![]))
        }
    }
}

#[derive(Deserialize)]
struct HistoryParams {
    range: Option<String>,
}

async fn get_historical_stats(
    Query(params): Query<HistoryParams>,
    axum::Extension(db): axum::Extension<Arc<db::Database>>,
) -> impl axum::response::IntoResponse {
    let range = params.range.unwrap_or_else(|| "24h".to_string());
    match db.get_historical_stats_by_range(&range).await {
        Ok(stats) => (axum::http::StatusCode::OK, Json(stats)),
        Err(e) => {
            tracing::error!("Fallo al obtener estadísticas históricas: {:?}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(vec![]))
        }
    }
}
