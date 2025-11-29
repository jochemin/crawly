extern crate bitcoin;
extern crate data_encoding;
extern crate hex;
extern crate reqwest;
extern crate serde_json;

use std::net::{Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpStream;

use arc_swap::ArcSwap;
use maxminddb::{geoip2, Reader};
use std::net::IpAddr;
#[derive(Clone)]
pub struct TypeAddress {
    pub address_type: String,
    pub address: String,
}

#[derive(Debug, Clone, Default)]
pub struct GeoIpInfo {
    pub country: String,
    pub city: String,
    pub latitude: f32,
    pub longitude: f32,
}

#[derive(Debug, Clone, Default)]
pub struct AsnInfo {
    pub isp: String,
    pub asn: u32,
}

pub fn first_nodes() -> Vec<SocketAddr> {
    let mut first_nodes = Vec::new();
    let seeds = vec![
        "seed.bitcoin.sipa.be:8333",
        "dnsseed.bluematt.me:8333",
        "dnsseed.bitcoin.dashjr.org:8333",
        "seed.bitcoinstats.com:8333",
        "seed.bitcoin.jonasschnelli.ch:8333",
        "seed.btc.petertodd.org:8333",
        "seed.bitcoin.sprovoost.nl:8333",
        "seed.bitcoin.sprovoost.nl:8333",
        "dnsseed.emzy.de:8333",
        "seed.bitcoin.wiz.biz:8333",
    ];
    for seed in seeds {
        let server_details = seed;
        let server: Vec<_> = server_details
            .to_socket_addrs()
            .expect("Unable to resolve domain")
            .collect();
        first_nodes.push(server);
    }
    let flatten_nodes = first_nodes
        .into_iter()
        .flatten()
        .collect::<Vec<SocketAddr>>();
    return flatten_nodes;
}

pub async fn scan_port_addr(addr: SocketAddr) -> bool {
    let timeout = Duration::from_secs(1);

    match tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}

pub fn type_address(addr: [u16; 8]) -> TypeAddress {
    let ipv6 = Ipv6Addr::from(addr);
    let onion = "fd87:d87e:eb43".to_string();
    match ipv6.to_ipv4() {
        Some(ip) => {
            return TypeAddress {
                address_type: "ipv4".to_string(),
                address: ip.to_string(),
            }
        }
        None => {
            if ipv6.to_string().contains(&onion) {
                return TypeAddress {
                    address_type: "onionV2".to_string(),
                    address: ipv6_to_onion(ipv6.to_string()),
                };
            } else {
                return TypeAddress {
                    address_type: "ipv6".to_string(),
                    address: ipv6.to_string(),
                };
            }
        }
    }
}

pub fn ipv6_to_onion(ipv6: String) -> String {
    let mut step1: Vec<_> = ipv6.split(":").collect();
    let _step2: Vec<_> = step1.drain(0..3).collect();
    let mut step3: String = "".to_string();
    for hextect in step1.iter_mut() {
        if hextect.len() != 4 {
            let good_hextect: String = format!("{:0>4}", hextect);
            step3 = step3 + &good_hextect;
        } else {
            step3 = step3 + &hextect.to_string();
        }
    }
    let encode = hex::decode(step3.as_bytes()).unwrap();
    let encode1 = data_encoding::BASE32.encode(&encode).to_lowercase() + ".onion";
    return encode1;
}

pub fn isprivate(addr: [u16; 8]) -> bool {
    let ipv6 = Ipv6Addr::from(addr);
    match ipv6.to_ipv4() {
        Some(ip) => return ip.is_private(),
        None => false,
    }
}

fn get_geoip_db_path(filename: &str) -> anyhow::Result<std::path::PathBuf> {
    let mut path = std::env::current_dir()?;
    path.push("database");
    path.push(filename);
    Ok(path)
}

#[derive(Clone)]
pub struct GeoIpReader {
    reader: Arc<ArcSwap<Reader<Vec<u8>>>>,
}

impl GeoIpReader {
    pub fn new() -> anyhow::Result<Self> {
        let path = get_geoip_db_path("GeoLite2-City.mmdb")?;
        let reader = Reader::open_readfile(path)?;
        Ok(Self {
            reader: Arc::new(ArcSwap::from(Arc::new(reader))),
        })
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<GeoIpInfo> {
        let reader_guard = self.reader.load();

        let lookup_result: Option<geoip2::City> = reader_guard.lookup(ip).ok().flatten();

        lookup_result.map(|geo_data| {
            let country = geo_data
                .country
                .and_then(|c| c.iso_code)
                .unwrap_or("")
                .to_string();
            let city = geo_data
                .city
                .and_then(|c| c.names.and_then(|mut n| n.remove("en")))
                .unwrap_or("")
                .to_string();
            let latitude = geo_data
                .location
                .as_ref()
                .and_then(|l| l.latitude)
                .unwrap_or(0.0) as f32;
            let longitude = geo_data
                .location
                .as_ref()
                .and_then(|l| l.longitude)
                .unwrap_or(0.0) as f32;

            GeoIpInfo {
                country,
                city,
                latitude,
                longitude,
            }
        })
    }

    pub fn reload(&self) -> anyhow::Result<()> {
        let path = get_geoip_db_path("GeoLite2-City.mmdb")?;
        tracing::info!(
            "[Mantenimiento] Recargando GeoIP-City desde {}",
            path.display()
        );
        let new_reader = Reader::open_readfile(path)?;
        self.reader.store(Arc::new(new_reader));
        Ok(())
    }
}

#[derive(Clone)]
pub struct GeoIpAsnReader {
    reader: Arc<ArcSwap<Reader<Vec<u8>>>>,
}

impl GeoIpAsnReader {
    pub fn new() -> anyhow::Result<Self> {
        let path = get_geoip_db_path("GeoLite2-ASN.mmdb")?;
        let reader = Reader::open_readfile(path)?;
        Ok(Self {
            reader: Arc::new(ArcSwap::from(Arc::new(reader))),
        })
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<AsnInfo> {
        let reader_guard = self.reader.load();
        let lookup_result: Option<geoip2::Asn> = reader_guard.lookup(ip).ok().flatten();

        lookup_result.map(|asn_data| {
            let isp = asn_data
                .autonomous_system_organization
                .unwrap_or("")
                .to_string();
            let asn = asn_data.autonomous_system_number.unwrap_or(0);

            AsnInfo { isp, asn }
        })
    }

    pub fn reload(&self) -> anyhow::Result<()> {
        let path = get_geoip_db_path("GeoLite2-ASN.mmdb")?;
        tracing::info!(
            "[Mantenimiento] Recargando GeoIP-ASN desde {}",
            path.display()
        );
        let new_reader = Reader::open_readfile(path)?;
        self.reader.store(Arc::new(new_reader));
        Ok(())
    }
}
