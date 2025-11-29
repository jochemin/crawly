use anyhow::{Context, Result};
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

#[derive(Serialize)]
pub struct NodeInfo {
    pub address: String,
    pub soft: Option<String>,
    pub country: Option<String>,
    pub detected: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct ClientCount {
    #[sqlx(default)]
    pub client: Option<String>,
    pub count: i64,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug, Clone)]
pub struct SoftwareCount {
    pub soft: Option<String>,
    pub node_count: i64,
}

#[derive(Serialize, Debug)]
pub struct ProtocolStats {
    pub protocol: String,
    pub total_nodes: i64,
    pub top_clients: Vec<ClientCount>,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct HourlyStat {
    pub snapshot_time: chrono::DateTime<chrono::Utc>,
    pub total_nodes: Option<i64>,
    pub incoming_nodes: Option<i64>,
    pub archive_nodes: Option<i64>,
    pub ipv4_nodes: Option<i64>,
    pub ipv6_nodes: Option<i64>,
    pub onion_nodes: Option<i64>,
    pub top_software: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct DiscoveredNode {
    pub addr_type: String,
    pub addr_str: String,
    pub port: u16,
    pub services: String,
}

#[derive(Clone)]
pub struct Database(pub sqlx::PgPool);

pub enum NodeToScan {
    Ip(SocketAddr),
    Tor { address: String, port: u16 },
    I2p { address: String, port: u16 },
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct SoftwareVersionStat {
    #[sqlx(default)]
    pub soft: Option<String>,
    pub node_count: Option<i64>,
}

impl Database {
    pub async fn new() -> Result<Self> {
        dotenvy::dotenv().ok();
        let db_url =
            std::env::var("DATABASE_URL").context("DATABASE_URL no encontrada en el entorno")?;

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&db_url)
            .await
            .context("No se pudo crear el pool de conexiones a PostgreSQL")?;

        Ok(Database(pool))
    }

    pub async fn seed_nodes(&self) -> Result<()> {
        let initial_nodes = crate::common::first_nodes();

        for node in initial_nodes {
            let addr_type = if node.is_ipv4() { "ipv4" } else { "ipv6" };
            let address = node.ip().to_string();
            let port = node.port();

            self.upsert_addrv2_node(addr_type, &address, port, "")
                .await?;
        }
        Ok(())
    }

    pub async fn get_nodes_to_scan(&self, limit: u32) -> Result<Vec<NodeToScan>> {
        let records = sqlx::query!(
            r#"
            SELECT address, port, type as "node_type"
            FROM bnetwork 
            WHERE (next_attempt_time < NOW() OR next_attempt_time IS NULL)
            AND type IN ('ipv4', 'ipv6', 'onionv3', 'i2p')
            ORDER BY next_attempt_time ASC NULLS FIRST
            LIMIT $1
            "#,
            limit as i64
        )
        .fetch_all(&self.0)
        .await
        .context("Fallo al obtener nodos para escanear")?;

        let nodes: Vec<NodeToScan> = records
            .into_iter()
            .filter_map(|row| {
                let addr = row.address.clone();
                let port = row.port.clone();
                let node_type = row.node_type.as_deref()?;

                match node_type {
                    "ipv4" | "ipv6" => {
                        if let (Ok(ip), Some(port_val)) = (addr.parse::<IpAddr>(), port) {
                            Some(NodeToScan::Ip(SocketAddr::new(ip, port_val as u16)))
                        } else {
                            tracing::warn!(
                                "No se pudo parsear la dirección IP o puerto: {} {}",
                                addr,
                                port.unwrap_or(-1)
                            );
                            None
                        }
                    }
                    "onionv2" | "onionv3" => {
                        if let Some(port_val) = port {
                            Some(NodeToScan::Tor {
                                address: addr,
                                port: port_val as u16,
                            })
                        } else {
                            tracing::warn!("Puerto faltante para nodo Tor: {}", addr);
                            None
                        }
                    }
                    "i2p" => {
                        if let Some(port_val) = port {
                            Some(NodeToScan::I2p {
                                address: addr,
                                port: port_val as u16,
                            })
                        } else {
                            tracing::warn!("Puerto faltante para nodo I2P: {}", addr);
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect();

        Ok(nodes)
    }

    pub async fn upsert_addrv2_node(
        &self,
        addr_type: &str,
        address: &str,
        port: u16,
        services: &str,
    ) -> Result<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            INSERT INTO bnetwork (address, type, port, services, added, detected)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (address) DO UPDATE SET
            detected = $6
            "#,
            address,
            addr_type,
            port as i32,
            services,
            now,
            now
        )
        .execute(&self.0)
        .await
        .context(format!("Fallo en el upsert del nodo {}", address))?;

        Ok(())
    }

    pub async fn update_ip_info(
        &self,
        ip: &str,
        country: &str,
        city: &str,
        latitude: f32,
        longitude: f32,
        isp: &str,
        asn: &str,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE bnetwork SET country = $1, city = $2, latitude = $3, longitude = $4, isp = $5, asn = $6 WHERE address = $7",
            country,
            city,
            latitude,
            longitude,
            isp,
            asn,
            ip
        )
        .execute(&self.0)
        .await
        .context(format!("Fallo al actualizar los detalles de la ip {}", ip))?;

        Ok(())
    }

    pub async fn clean_db(&self) -> Result<()> {
        let result = sqlx::query!(
            r#"
        DELETE FROM bnetwork 
        WHERE 
            detected < NOW() - INTERVAL '2 days'
        "#
        )
        .execute(&self.0)
        .await
        .context("Error al limpiar la BBDD (clean_db)")?;

        tracing::info!(
            "[Mantenimiento] Limpieza de BBDD completada. Se eliminaron {} nodos antiguos.",
            result.rows_affected()
        );

        Ok(())
    }

    pub async fn ip_info_list(&self) -> Result<Vec<String>> {
        let ips = sqlx::query_scalar!(
            "SELECT address FROM bnetwork WHERE type LIKE '%ipv%' AND country IS NULL LIMIT 2000"
        )
        .fetch_all(&self.0)
        .await
        .context("Fallo al obtener la lista de IPs para enriquecer")?;

        Ok(ips)
    }

    pub async fn update_handshake_info(
        &self,
        address_with_port: &str,
        agent: &str,
        services: &str,
        protocol_version: i32,
        start_height: i32,
        relay: bool,
    ) -> Result<()> {
        let address = if let Some((host, _port)) = address_with_port.rsplit_once(':') {
            host
        } else {
            address_with_port
        };
        let now = Utc::now();
        let address_str = address.to_string();
        let address_str = address_str.trim_matches(|c| c == '[' || c == ']');

        sqlx::query!(
            r#"
            UPDATE bnetwork
            SET
                scanned = $1,
                soft = $2,
                services = $3,
                protocol_version = $4,
                start_height = $5,
                relay = $6,
                incoming = TRUE
            WHERE address = $7
            "#,
            now,
            agent,
            services,
            protocol_version,
            start_height,
            relay,
            address_str
        )
        .execute(&self.0)
        .await
        .context(format!(
            "Fallo al actualizar info del handshake para {}",
            address_str
        ))?;

        Ok(())
    }
    pub async fn get_total_nodes_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM bnetwork")
            .fetch_one(&self.0)
            .await?
            .unwrap_or(0);
        Ok(count)
    }
    pub async fn get_recent_nodes(&self, limit: i64, offset: i64) -> Result<Vec<NodeInfo>> {
        let nodes = sqlx::query_as!(
            NodeInfo,
            "SELECT address, soft, country, detected
            FROM bnetwork
            WHERE scanned IS NOT NULL
            ORDER BY scanned DESC
            LIMIT $1 OFFSET $2",
            limit,
            offset
        )
        .fetch_all(&self.0)
        .await
        .context("Fallo al obtener nodos recientes")?;

        Ok(nodes)
    }
    pub async fn find_node_by_address(&self, address: &str) -> Result<Option<NodeInfo>> {
        let node = sqlx::query_as!(
            NodeInfo,
            "SELECT address, soft, country, detected
            FROM bnetwork
            WHERE address = $1",
            address
        )
        .fetch_optional(&self.0)
        .await
        .context("Fallo al buscar nodo por dirección")?;

        Ok(node)
    }

    pub async fn get_archive_nodes_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM bnetwork WHERE services ~ '(NETWORK\||NETWORK\))' AND incoming = TRUE"#
        )
        .fetch_one(&self.0)
        .await?
        .unwrap_or(0);
        Ok(count)
    }

    pub async fn get_ipv4_nodes_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM bnetwork WHERE type = 'ipv4' AND incoming = TRUE"#
        )
        .fetch_one(&self.0)
        .await?
        .unwrap_or(0);
        Ok(count)
    }
    pub async fn get_ipv6_nodes_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM bnetwork WHERE type = 'ipv6' AND incoming = TRUE"#
        )
        .fetch_one(&self.0)
        .await?
        .unwrap_or(0);
        Ok(count)
    }
    pub async fn get_tor_nodes_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM bnetwork  WHERE type IN ('onionv2', 'onionv3') AND incoming = TRUE"#
        )
        .fetch_one(&self.0)
        .await?
        .unwrap_or(0);
        Ok(count)
    }
    pub async fn get_knots_nodes_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM bnetwork  WHERE soft LIKE '%Knots%' AND incoming = TRUE"#
        )
        .fetch_one(&self.0)
        .await?
        .unwrap_or(0);
        Ok(count)
    }
    pub async fn get_core30_nodes_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM bnetwork  WHERE soft LIKE '/Satoshi:30%' AND incoming = TRUE"#
        )
        .fetch_one(&self.0)
        .await?
        .unwrap_or(0);
        Ok(count)
    }

    pub async fn handle_successful_connection(&self, address_with_port: &str) -> Result<()> {
        let now = Utc::now();
        let address_part = if let Some((host, _port)) = address_with_port.rsplit_once(':') {
            host
        } else {
            address_with_port
        };
        let address_str = address_part
            .trim_matches(|c| c == '[' || c == ']')
            .to_string();

        let node_type_result =
            sqlx::query_scalar!("SELECT type FROM bnetwork WHERE address = $1", address_str)
                .fetch_optional(&self.0)
                .await;

        let node_type: Option<String> = match node_type_result {
            Ok(opt_opt_string) => opt_opt_string.flatten(),
            Err(e) => {
                tracing::error!("Failed to fetch node type for {}: {}", address_str, e);
                return Err(e.into());
            }
        };

        let next_attempt = match node_type.as_deref() {
            Some("onionv2") | Some("onionv3") | Some("i2p") => {
                let mut rng = rand::rng();
                let random_hours = rng.random_range(8..=15);
                let random_minutes = rng.random_range(0..=59);
                now + chrono::Duration::hours(random_hours)
                    + chrono::Duration::minutes(random_minutes)
            }
            _ => now + chrono::Duration::hours(12),
        };

        sqlx::query!(
            r#"
            UPDATE bnetwork 
            SET 
                scanned = $1, 
                incoming = TRUE,
                consecutive_failures = 0,
                reliability_score = reliability_score + 1,
                next_attempt_time = $2
            WHERE address = $3
            "#,
            now,
            next_attempt,
            address_str
        )
        .execute(&self.0)
        .await
        .context(format!(
            "Fallo al manejar conexión exitosa para {}",
            address_str
        ))?;

        Ok(())
    }

    pub async fn handle_failed_connection(&self, address_with_port: &str) -> Result<()> {
        let address_part = if let Some((host, _port)) = address_with_port.rsplit_once(':') {
            host
        } else {
            address_with_port
        };
        let address = address_part.trim_matches(|c| c == '[' || c == ']');

        let node_info: Option<(String, i32)> =
            sqlx::query_as("SELECT type, consecutive_failures FROM bnetwork WHERE address = $1")
                .bind(address)
                .fetch_optional(&self.0)
                .await?;

        let (node_type, current_failures) =
            if let Some((fetched_type, fetched_failures)) = node_info {
                (fetched_type, fetched_failures)
            } else {
                tracing::debug!(
                    "Fallo registrado para un nodo no existente en la BBDD: {}",
                    address
                );
                return Ok(());
            };

        let new_failures = current_failures + 1;

        let next_attempt = {
            let mut rng = rand::rng();
            let random_seconds = rng.random_range((4 * 60 * 60)..=(24 * 60 * 60));
            Utc::now() + chrono::Duration::seconds(random_seconds)
        };

        let set_incoming: Option<bool> = if new_failures >= 3 { Some(false) } else { None };
        let now = Utc::now();
        sqlx::query!(
            r#"
                UPDATE bnetwork SET
                    consecutive_failures = $1,
                    reliability_score = reliability_score - 1,
                    next_attempt_time = $2,
                    incoming = COALESCE($3, incoming),
                    scanned = $4
                WHERE address = $5
                "#,
            new_failures,
            next_attempt,
            set_incoming,
            now,
            address
        )
        .execute(&self.0)
        .await
        .context(format!(
            "Fallo al actualizar estado tras fallo para {}",
            address
        ))?;

        if node_type.contains("onion") && new_failures >= 3 {
            tracing::warn!(
                "[Pruning] El nodo Tor {} ha fallado {} veces. Eliminando de la BBDD.",
                address,
                new_failures
            );
            sqlx::query!("DELETE FROM bnetwork WHERE address = $1", address)
                .execute(&self.0)
                .await
                .context(format!("Fallo al borrar nodo Tor {}", address))?;
        }

        Ok(())
    }

    pub async fn get_software_version_stats(&self) -> Result<Vec<SoftwareVersionStat>> {
        let stats = sqlx::query_as!(
            SoftwareVersionStat,
            r#"
            SELECT soft, COUNT(*) as node_count
            FROM bnetwork
            WHERE soft IS NOT NULL AND soft != '' AND incoming = TRUE
            GROUP BY soft
            ORDER BY node_count DESC
            "#
        )
        .fetch_all(&self.0)
        .await
        .context("Fallo al obtener estadísticas de versiones de software")?;

        Ok(stats)
    }

    pub async fn get_top_software_stats(&self) -> Result<Vec<SoftwareCount>> {
        let stats = sqlx::query_as!(
            SoftwareCount,
            r#"
            SELECT
                soft,
                COUNT(*) as "node_count!"
            FROM bnetwork
            WHERE soft IS NOT NULL AND soft != '' AND incoming = TRUE
            GROUP BY soft
            ORDER BY COUNT(*) DESC
            LIMIT 10
            "#
        )
        .fetch_all(&self.0)
        .await
        .context("Fallo al obtener top 10 estadísticas de software")?;

        Ok(stats)
    }

    pub async fn get_historical_stats_by_range(&self, range: &str) -> Result<Vec<HourlyStat>> {
        let (interval, filter_condition) = match range {
            "1w" => ("'7 days'", "EXTRACT(HOUR FROM snapshot_time) % 6 = 0"),
            "1m" => ("'1 month'", "EXTRACT(HOUR FROM snapshot_time) = 0"),
            "1y" => (
                "'1 year'",
                "EXTRACT(DOW FROM snapshot_time) = 0 AND EXTRACT(HOUR FROM snapshot_time) = 0",
            ),
            _ => ("'24 hours'", "TRUE"),
        };

        let query = format!(
            r#"
            SELECT 
                snapshot_time, 
                total_nodes, 
                incoming_nodes, 
                archive_nodes, 
                ipv4_nodes, 
                ipv6_nodes, 
                onion_nodes, 
                top_software
            FROM hourly_stats
            WHERE snapshot_time > NOW() - INTERVAL {} 
            AND {}
            ORDER BY snapshot_time ASC
            "#,
            interval, filter_condition
        );

        let stats = sqlx::query_as::<_, HourlyStat>(&query)
            .fetch_all(&self.0)
            .await
            .context("Fallo al obtener estadísticas históricas por rango")?;

        Ok(stats)
    }

    pub async fn update_inbound_node_info(
        &self,
        address_with_port: &str,
        agent: &str,
        services: &str,
        protocol_version: i32,
        start_height: i32,
        relay: bool,
    ) -> Result<()> {
        let address_part = if let Some((host, _port)) = address_with_port.rsplit_once(':') {
            host
        } else {
            address_with_port
        };
        let address_str = address_part
            .trim_matches(|c| c == '[' || c == ']')
            .to_string();

        sqlx::query!(
            r#"
            UPDATE bnetwork
            SET
                soft = $1,
                services = $2,
                protocol_version = $3,
                start_height = $4,
                relay = $5
            WHERE address = $6
            "#,
            agent,
            services,
            protocol_version,
            start_height,
            relay,
            address_str
        )
        .execute(&self.0)
        .await
        .context(format!(
            "Fallo al actualizar info (inbound) para {}",
            address_str
        ))?;

        Ok(())
    }
    pub async fn get_incoming_stats_by_protocol(&self) -> Result<Vec<ProtocolStats>> {
        let mut final_stats: Vec<ProtocolStats> = Vec::new();

        let simple_protocols = vec!["ipv4", "ipv6", "i2p", "cjdns", "yggdrasil"];

        for proto in simple_protocols {
            let total = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM bnetwork WHERE incoming = TRUE AND type = $1",
                proto
            )
            .fetch_one(&self.0)
            .await?
            .unwrap_or(0);

            let clients = sqlx::query_as!(
                ClientCount,
                r#"
                    SELECT soft as "client", COUNT(*) as "count!"
                    FROM bnetwork
                    WHERE 
                        incoming = TRUE 
                        AND type = $1 
                        AND soft IS NOT NULL AND soft != ''
                    GROUP BY soft 
                    ORDER BY COUNT(*) DESC
                    LIMIT 10
                    "#,
                proto
            )
            .fetch_all(&self.0)
            .await?;

            final_stats.push(ProtocolStats {
                protocol: proto.to_string(),
                total_nodes: total,
                top_clients: clients,
            });
        }

        let onion_total = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM bnetwork WHERE incoming = TRUE AND type IN ('onionv2', 'onionv3')"
            )
            .fetch_one(&self.0)
            .await?
            .unwrap_or(0);

        let onion_clients = sqlx::query_as!(
            ClientCount,
            r#"
                SELECT soft as "client", COUNT(*) as "count!"
                FROM bnetwork
                WHERE 
                    incoming = TRUE 
                    AND type IN ('onionv2', 'onionv3') 
                    AND soft IS NOT NULL AND soft != ''
                GROUP BY soft 
                ORDER BY COUNT(*) DESC
                LIMIT 10
                "#
        )
        .fetch_all(&self.0)
        .await?;

        final_stats.push(ProtocolStats {
            protocol: "onion".to_string(),
            total_nodes: onion_total,
            top_clients: onion_clients,
        });

        Ok(final_stats)
    }

    pub async fn batch_upsert_addrv2_nodes(&self, nodes: &[DiscoveredNode]) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        let mut types = Vec::with_capacity(nodes.len());
        let mut addresses = Vec::with_capacity(nodes.len());
        let mut ports = Vec::with_capacity(nodes.len());
        let mut services = Vec::with_capacity(nodes.len());

        for node in nodes {
            types.push(node.addr_type.clone());
            addresses.push(node.addr_str.clone());
            ports.push(node.port as i32);
            services.push(node.services.clone());
        }
        let now = Utc::now();

        sqlx::query!(
            r#"
            INSERT INTO bnetwork (address, type, port, services, added, detected)
            SELECT 
                u.address, 
                u.type, 
                u.port, 
                u.services, 
                $5,
                $5
            FROM UNNEST(
                $1::text[], $2::text[], $3::int4[], $4::text[]
            ) AS u(address, type, port, services)
            ON CONFLICT (address) DO UPDATE SET
                detected = $5
            "#,
            &addresses[..],
            &types[..],
            &ports[..],
            &services[..],
            now
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }
    pub async fn get_incoming_nodes_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM bnetwork WHERE incoming = TRUE"#)
            .fetch_one(&self.0)
            .await?
            .unwrap_or(0);
        Ok(count)
    }

    pub async fn search_nodes(&self, query: &str) -> Result<Vec<NodeInfo>> {
        let pattern = format!("%{}%", query);
        let nodes = sqlx::query_as!(
            NodeInfo,
            "SELECT address, soft, country, detected
            FROM bnetwork
            WHERE address ILIKE $1 OR soft ILIKE $1
            ORDER BY detected DESC NULLS LAST
            LIMIT 50",
            pattern
        )
        .fetch_all(&self.0)
        .await
        .context("Fallo al buscar nodos")?;

        Ok(nodes)
    }
}
