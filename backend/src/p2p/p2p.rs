extern crate bitcoin;
extern crate data_encoding;
extern crate rand;

use crate::common::type_address;
use anyhow::{Context, Result};
use rand::Rng;

use std::convert::TryInto;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use self::bitcoin::consensus::{deserialize, serialize};
use self::bitcoin::p2p::{address, message, message_network, ServiceFlags};
use bitcoin::Network;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

use sha3::{Digest, Sha3_256};
use std::collections::HashMap;

async fn handle_stream<S>(
    db: &crate::db::Database,
    address_str: String,
    mut stream: S,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    loop {
        let mut header_buf = [0u8; 24];

        if let Err(e) = stream.read_exact(&mut header_buf).await {
            tracing::warn!(target: "p2p", "Fallo al leer el header desde {}: {}. Conexión cerrada.", address_str, e);
            return Err(anyhow::Error::from(e)
                .context(format!("Fallo al leer header desde {}", address_str)));
        }

        let payload_size_bytes: [u8; 4] = header_buf[16..20]
            .try_into()
            .context("No se pudieron leer los bytes del tamaño del payload")?;
        let payload_size = u32::from_le_bytes(payload_size_bytes) as usize;

        if payload_size > 1_000_000 {
            tracing::warn!(target: "p2p", "Payload demasiado grande ({}) recibido desde {}. Descartando.", payload_size, address_str);
            return Err(anyhow::anyhow!(
                "Payload demasiado grande ({}) desde {}",
                payload_size,
                address_str
            ));
        }

        let mut payload_buf = vec![0u8; payload_size];
        if let Err(e) = stream.read_exact(&mut payload_buf).await {
            tracing::warn!(target: "p2p", "Fallo al leer el payload desde {}: {}. Conexión cerrada.", address_str, e);
            return Err(anyhow::Error::from(e)
                .context(format!("Fallo al leer payload desde {}", address_str)));
        }

        let mut full_message_buf = Vec::with_capacity(24 + payload_size);
        full_message_buf.extend_from_slice(&header_buf);
        full_message_buf.extend_from_slice(&payload_buf);

        let reply: message::RawNetworkMessage = match deserialize(&full_message_buf) {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!(target: "p2p", "Mensaje inválido recibido de {}: {}. Ignorando.", address_str, e);
                return Err(anyhow::Error::from(e)
                    .context(format!("Mensaje inválido desde {}", address_str)));
            }
        };

        tracing::debug!(target: "p2p", "Recibido mensaje '{}' de {}", reply.payload().cmd(), address_str);

        match reply.payload() {
            message::NetworkMessage::Version(x) => {
                let soft = &x.user_agent;
                let services = &x.services.to_string();
                let protocol_version = x.version as i32;
                let start_height = x.start_height;
                let relay = x.relay;

                if let Err(e) = db.handle_successful_connection(&address_str).await {
                    tracing::error!("Fallo de BBDD (handle_success) para {}: {}", address_str, e);
                }

                if let Err(e) = db
                    .update_handshake_info(
                        &address_str,
                        soft,
                        services,
                        protocol_version,
                        start_height,
                        relay,
                    )
                    .await
                {
                    tracing::error!("Fallo de BBDD (handshake_info) para {}: {}", address_str, e);
                }

                let sendaddrv2_message = message::RawNetworkMessage::new(
                    Network::Bitcoin.magic(),
                    message::NetworkMessage::SendAddrV2,
                );
                stream
                    .write_all(serialize(&sendaddrv2_message).as_slice())
                    .await
                    .context("Fallo al enviar 'sendaddrv2'")?;

                let second_message = message::RawNetworkMessage::new(
                    Network::Bitcoin.magic(),
                    message::NetworkMessage::Verack,
                );
                stream
                    .write_all(serialize(&second_message).as_slice())
                    .await
                    .context("Fallo al enviar 'verack'")?;
            }
            message::NetworkMessage::Verack => {
                tracing::info!(target: "p2p", "Handshake completado con {}", address_str);
                let getaddr_message = message::RawNetworkMessage::new(
                    Network::Bitcoin.magic(),
                    message::NetworkMessage::GetAddr,
                );
                stream
                    .write_all(serialize(&getaddr_message).as_slice())
                    .await
                    .context("Fallo al enviar 'getaddr'")?;
            }
            message::NetworkMessage::Ping(nonce) => {
                let pong_message = message::RawNetworkMessage::new(
                    Network::Bitcoin.magic(),
                    message::NetworkMessage::Pong(*nonce),
                );
                stream
                    .write_all(serialize(&pong_message).as_slice())
                    .await
                    .context("Fallo al enviar 'pong'")?;
            }
            message::NetworkMessage::Addr(ref x) => {
                tracing::info!(target: "p2p", "Recibido mensaje Addr con {} direcciones de {}", x.len(), address_str);
                for addr in x {
                    let type_addr = type_address(addr.1.address);
                    let services_str = addr.1.services.to_string();
                    if let Err(e) = db
                        .upsert_addrv2_node(
                            &type_addr.address_type,
                            &type_addr.address,
                            addr.1.port,
                            &services_str,
                        )
                        .await
                    {
                        tracing::error!("Fallo al insertar nodo Addr {}: {}", type_addr.address, e);
                    }
                }
                break;
            }
            message::NetworkMessage::AddrV2(ref addrv2_messages) => {
                tracing::info!(target: "p2p", "Recibido AddrV2 ({} nodos) de {}", addrv2_messages.len(), address_str);

                let messages_to_process: Vec<_> = addrv2_messages.iter().cloned().collect();
                let db_clone = db.clone();

                tokio::spawn(async move {
                    let mut nodes_to_insert: HashMap<String, crate::db::DiscoveredNode> =
                        HashMap::with_capacity(messages_to_process.len());

                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs() as u32;

                    const FORTY_EIGHT_HOURS: u32 = 48 * 60 * 60;
                    const TEN_MINUTES: u32 = 10 * 60;
                    const MAX_FUTURE: u32 = 60 * 60;

                    let mut filtered_count = 0;
                    let mut future_count = 0;
                    let mut attack_count = 0;

                    for entry in messages_to_process {
                        let node_time = entry.time;

                        if node_time > now + MAX_FUTURE {
                            tracing::debug!(target: "p2p", 
                                "Descartando nodo con timestamp muy lejano en el futuro: {} (ahora: {})", 
                                node_time, now);
                            attack_count += 1;
                            continue;
                        }

                        let effective_time = if node_time > now && node_time <= now + TEN_MINUTES {
                            future_count += 1;
                            now
                        } else {
                            node_time
                        };

                        if now.saturating_sub(effective_time) > FORTY_EIGHT_HOURS {
                            filtered_count += 1;
                            continue;
                        }

                        let (addr_type, addr_str) = match &entry.addr {
                            bitcoin::p2p::address::AddrV2::Ipv4(addr) => {
                                ("ipv4".to_string(), addr.to_string())
                            }
                            bitcoin::p2p::address::AddrV2::Ipv6(addr) => {
                                ("ipv6".to_string(), addr.to_string())
                            }
                            bitcoin::p2p::address::AddrV2::TorV2(bytes) => {
                                let mut spec = data_encoding::BASE32.specification();
                                spec.padding = None;
                                match spec.encoding() {
                                    Ok(encoding) => {
                                        let addr = encoding.encode(bytes).to_lowercase() + ".onion";
                                        ("onionv2".to_string(), addr)
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Fallo al crear encoder Base32 para TorV2: {}",
                                            e
                                        );
                                        continue;
                                    }
                                }
                            }
                            bitcoin::p2p::address::AddrV2::TorV3(bytes) => {
                                let addr_str = match build_torv3_address(bytes) {
                                    Ok(addr) => addr,
                                    Err(e) => {
                                        tracing::error!(
                                            "Fallo al construir la dirección TorV3: {}",
                                            e
                                        );
                                        continue;
                                    }
                                };
                                ("onionv3".to_string(), addr_str)
                            }
                            bitcoin::p2p::address::AddrV2::I2p(bytes) => {
                                let mut spec = data_encoding::BASE32.specification();
                                spec.padding = None;
                                match spec.encoding() {
                                    Ok(encoding) => {
                                        let addr =
                                            encoding.encode(bytes).to_lowercase() + ".b32.i2p";
                                        ("i2p".to_string(), addr)
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Fallo al crear encoder Base32 para I2P: {}",
                                            e
                                        );
                                        continue;
                                    }
                                }
                            }
                            bitcoin::p2p::address::AddrV2::Cjdns(addr_bytes) => {
                                let ipv6_addr = std::net::Ipv6Addr::from(*addr_bytes);
                                ("cjdns".to_string(), ipv6_addr.to_string())
                            }
                            bitcoin::p2p::address::AddrV2::Unknown(network_id, bytes) => {
                                if *network_id == 0x07 && bytes.len() == 16 {
                                    match <&[u8] as TryInto<[u8; 16]>>::try_into(bytes.as_slice()) {
                                        Ok(addr_bytes_16) => {
                                            let ipv6_addr = std::net::Ipv6Addr::from(addr_bytes_16);
                                            ("yggdrasil".to_string(), ipv6_addr.to_string())
                                        }
                                        Err(_) => {
                                            tracing::warn!("Recibido tipo Unknown (0x07) pero no son 16 bytes válidos.");
                                            continue;
                                        }
                                    }
                                } else {
                                    tracing::debug!("Recibido tipo de dirección AddrV2 desconocido o no soportado: ID={}, Longitud={}", network_id, bytes.len());
                                    continue;
                                }
                            }
                        };

                        let mut port_to_store = entry.port;
                        if port_to_store == 0 && (addr_type == "i2p" || addr_type == "onionv2") {
                            port_to_store = 8333;
                        }

                        let node_to_store = crate::db::DiscoveredNode {
                            addr_type,
                            addr_str: addr_str.clone(),
                            port: port_to_store,
                            services: entry.services.to_string(),
                        };

                        nodes_to_insert.insert(addr_str, node_to_store);

                        if filtered_count > 0 || future_count > 0 || attack_count > 0 {
                            tracing::info!(target: "p2p", 
                            "AddrV2 filtrado: {} nodos aceptados, {} filtrados (>48h), {} normalizados (futuro cercano), {} rechazados (ataque/error)",
                            nodes_to_insert.len(), filtered_count, future_count, attack_count);
                        }
                    }

                    if !nodes_to_insert.is_empty() {
                        let nodes_vec: Vec<_> = nodes_to_insert.into_values().collect();
                        let chunk_size = 50;

                        for chunk in nodes_vec.chunks(chunk_size) {
                            let max_retries = 3;
                            let mut attempts = 0;
                            loop {
                                attempts += 1;
                                match db_clone.batch_upsert_addrv2_nodes(chunk).await {
                                    Ok(_) => break,
                                    Err(e) => {
                                        if let Some(db_err) = e.downcast_ref::<sqlx::Error>() {
                                            if let sqlx::Error::Database(db_err_info) = db_err {
                                                if db_err_info
                                                    .code()
                                                    .map_or(false, |code| code == "40P01")
                                                {
                                                    if attempts < max_retries {
                                                        tracing::warn!(target: "p2p", "Deadlock detectado en chunk upsert (intento {}/{}). Reintentando...", attempts, max_retries);
                                                        let delay =
                                                            tokio::time::Duration::from_millis(
                                                                rand::rng().random_range(100..=600),
                                                            );
                                                        tokio::time::sleep(delay).await;
                                                        continue;
                                                    } else {
                                                        tracing::error!("Fallo en chunk upsert tras {} intentos (deadlock): {}", attempts, e);
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        tracing::error!("Fallo irrecuperable en chunk upsert (tarea AddrV2): {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                break;
            }
            _ => {}
        }
    }

    Ok(())
}

pub async fn converse(db: &crate::db::Database, address: SocketAddr) -> Result<()> {
    let conversation_timeout = Duration::from_secs(60);

    let task = async move {
        let connect_timeout = Duration::from_secs(5);

        let connect_future = TcpStream::connect(&address);
        let mut stream = tokio::time::timeout(connect_timeout, connect_future)
            .await
            .context(format!("Timeout al conectar con {}", address))?
            .context(format!("Fallo al conectar con {}", address))?;

        let version_message = build_version_message(address)?;
        let first_message =
            message::RawNetworkMessage::new(Network::Bitcoin.magic(), version_message);
        stream
            .write_all(serialize(&first_message).as_slice())
            .await
            .context("Fallo al enviar el mensaje 'version'")?;

        tracing::info!(target: "p2p", "Enviado mensaje 'version' a {}. Esperando respuesta...", address);

        handle_stream(db, address.to_string(), stream).await
    };

    match tokio::time::timeout(conversation_timeout, task).await {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!(
            "Timeout global de la conversación con {}",
            address
        )),
    }
}

pub async fn converse_tor(db: &crate::db::Database, onion_address: &str, port: u16) -> Result<()> {
    let proxy_addr = "127.0.0.1:9050";
    let conversation_timeout = Duration::from_secs(60);
    let full_address = format!("{}:{}", onion_address, port);
    let full_address_clone = full_address.clone();

    let task = async move {
        let connect_timeout = Duration::from_secs(30);

        let connect_future = Socks5Stream::connect(proxy_addr, (onion_address, port));
        let mut stream = tokio::time::timeout(connect_timeout, connect_future)
            .await
            .context(format!("Timeout al conectar a {} vía Tor", full_address))?
            .context(format!("Fallo al conectar a {} vía Tor", full_address))?;

        let dummy_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let version_message = build_version_message(dummy_addr)?;
        let first_message =
            message::RawNetworkMessage::new(Network::Bitcoin.magic(), version_message);
        stream
            .write_all(serialize(&first_message).as_slice())
            .await
            .context(format!("Fallo al enviar 'version' a {}", full_address))?;
        handle_stream(db, full_address.clone(), stream).await
    };

    match tokio::time::timeout(conversation_timeout, task).await {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!(
            "Timeout global de la conversación con {}",
            full_address_clone
        )),
    }
}

pub async fn converse_i2p(db: &crate::db::Database, onion_address: &str, port: u16) -> Result<()> {
    let proxy_addr = "127.0.0.1:4446";
    let conversation_timeout = Duration::from_secs(60);
    let full_address = format!("{}:{}", onion_address, port);
    let full_address_clone = full_address.clone();

    let task = async move {
        let connect_timeout = Duration::from_secs(30);

        let connect_future = Socks5Stream::connect(proxy_addr, (onion_address, port));
        let mut stream = tokio::time::timeout(connect_timeout, connect_future)
            .await
            .context(format!("Timeout al conectar a {} vía I2P", full_address))?
            .context(format!("Fallo al conectar a {} vía I2P", full_address))?;

        let dummy_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let version_message = build_version_message(dummy_addr)?;
        let first_message =
            message::RawNetworkMessage::new(Network::Bitcoin.magic(), version_message);
        stream
            .write_all(serialize(&first_message).as_slice())
            .await
            .context(format!("Fallo al enviar 'version' a {}", full_address))?;
        handle_stream(db, full_address.clone(), stream).await
    };

    match tokio::time::timeout(conversation_timeout, task).await {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!(
            "Timeout global de la conversación con {}",
            full_address_clone
        )),
    }
}

pub async fn handle_connection(
    db: &crate::db::Database,
    mut stream: TcpStream,
    peer_addr: SocketAddr,
) -> Result<()> {
    tracing::debug!(target: "p2p", "Esperando mensaje 'version' de {}", peer_addr);

    let mut header_buf = [0u8; 24];
    if let Err(_) =
        tokio::time::timeout(Duration::from_secs(30), stream.read_exact(&mut header_buf)).await
    {
        tracing::warn!(target: "p2p", "Timeout esperando el primer mensaje de {}. Cerrando.", peer_addr);
        return Ok(());
    }

    let payload_size = u32::from_le_bytes(header_buf[16..20].try_into()?) as usize;

    if payload_size > 1024 {
        tracing::warn!(target: "p2p", "El primer mensaje de {} tiene un payload de {} bytes. Demasiado grande. Descartando.", peer_addr, payload_size);
        return Ok(());
    }

    let mut payload_buf = vec![0u8; payload_size];
    if stream.read_exact(&mut payload_buf).await.is_err() {
        tracing::warn!(target: "p2p", "Fallo al leer payload desde {}. Cerrando.", peer_addr);
        return Ok(());
    }

    let full_message_buf = [&header_buf[..], &payload_buf[..]].concat();
    let reply: message::RawNetworkMessage = match deserialize(&full_message_buf) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!(target: "p2p", "Mensaje inválido recibido de {}: {} ({:?}). Ignorando.", peer_addr, e, e);
            return Err(
                anyhow::Error::from(e).context(format!("Mensaje inválido desde {}", peer_addr))
            );
        }
    };

    if let message::NetworkMessage::Version(version_msg) = reply.payload() {
        let soft = &version_msg.user_agent;
        let services = &version_msg.services.to_string();
        let protocol_version = version_msg.version as i32;
        let start_height = version_msg.start_height;
        let relay = version_msg.relay;

        tracing::info!(target: "p2p", "Recibido 'version' de {}: user_agent='{}'", peer_addr, soft);

        db.update_inbound_node_info(
            peer_addr.to_string().as_str(),
            soft,
            services,
            protocol_version,
            start_height,
            relay,
        )
        .await?;
    } else {
        tracing::debug!(target: "p2p", "El primer mensaje de {} no fue 'version', sino '{}'. Ignorando.", peer_addr, reply.command());
    }

    Ok(())
}

fn build_version_message(address: SocketAddr) -> Result<message::NetworkMessage> {
    let my_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(170, 75, 173, 236)), 8333);
    let services = ServiceFlags::NETWORK | ServiceFlags::WITNESS;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("El tiempo del sistema es anterior a la época UNIX. ¿Estás en un Delorean?")?
        .as_secs();

    let addr_recv = address::Address::new(&address, ServiceFlags::NONE);
    let addr_from = address::Address::new(&my_address, services);
    let nonce: u64 = rand::random();
    let user_agent = String::from("Crawly");
    let start_height: i32 = 0;

    Ok(message::NetworkMessage::Version(
        message_network::VersionMessage::new(
            services,
            timestamp as i64,
            addr_recv,
            addr_from,
            nonce,
            user_agent,
            start_height,
        ),
    ))
}

fn build_torv3_address(pubkey: &[u8; 32]) -> Result<String> {
    const VERSION: u8 = 0x03;
    const CHECKSUM_PREFIX: &[u8] = b".onion checksum";

    let mut hasher = Sha3_256::new();
    hasher.update(CHECKSUM_PREFIX);
    hasher.update(pubkey);
    hasher.update(&[VERSION]);

    let hash = hasher.finalize();
    let checksum: [u8; 2] = hash[..2]
        .try_into()
        .context("Fallo al truncar el checksum")?;

    let mut full_data = [0u8; 35];
    full_data[..32].copy_from_slice(pubkey);
    full_data[32..34].copy_from_slice(&checksum);
    full_data[34] = VERSION;

    let mut spec = data_encoding::BASE32.specification();
    spec.padding = None;
    let encoding = spec
        .encoding()
        .context("Fallo al crear el encoder BASE32")?;

    let address = encoding.encode(&full_data).to_lowercase();

    Ok(address + ".onion")
}
