extern crate bitcoin;
extern crate reqwest;
extern crate serde_json;
extern crate data_encoding;
extern crate hex;

use std::{net::{Ipv6Addr, SocketAddr, TcpStream, ToSocketAddrs}, path::PathBuf}; 
use std::time::Duration;
use std::{env, io};
use std::{thread, time};

use serde_json::{Result, Value};
pub struct TypeAddress {
    pub address_type:String,
    pub address:String
}

//Función para coger nodos desde las semillas. Devuelve un listado.
pub fn first_nodes() -> Vec<SocketAddr>{
    let mut first_nodes = Vec::new();
    let seeds = vec!
        ["seed.bitcoin.sipa.be:8333", 
        "dnsseed.bluematt.me:8333", 
        "dnsseed.bitcoin.dashjr.org:8333", 
        "seed.bitcoinstats.com:8333", 
        "seed.bitcoin.jonasschnelli.ch:8333", 
        "seed.btc.petertodd.org:8333",  
        "seed.bitcoin.sprovoost.nl:8333", 
        "seed.bitcoin.sprovoost.nl:8333", 
        "dnsseed.emzy.de:8333", 
        "seed.bitcoin.wiz.biz:8333"];
    for seed in seeds{
        let server_details = seed;
        let server: Vec<_> = server_details
            .to_socket_addrs()
            .expect("Unable to resolve domain")
            .collect();
        first_nodes.push(server);
    }
    let flatten_nodes = first_nodes.into_iter().flatten().collect::<Vec<SocketAddr>>();
    return flatten_nodes;
}

//Función para comprobar si el nodo acepta conexiones entrantes
pub fn scan_port_addr(addr: SocketAddr) -> bool {
    let timeout = Duration::from_secs(1);
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => true,
        Err(_) => false,
    }
}

//Tratamiento addr
pub fn type_address(addr:[u16; 8]) -> TypeAddress {
    let ipv6 = Ipv6Addr::from(addr);
    //println!("{}", ipv6.to_string());
    //fd87:d87e:eb43:4bb3:51d5:9fe8:8e07:94a2
    let onion = "fd87:d87e:eb43".to_string();
    match ipv6.to_ipv4() {
        Some(ip) => return TypeAddress{address_type: "ipv4".to_string(), address: ip.to_string()},
        None => if ipv6.to_string().contains(&onion) {
                    return TypeAddress{address_type: "onionV2".to_string(), address: ipv6_to_onion( ipv6.to_string())}}
                else{
                    return TypeAddress{address_type: "ipv6".to_string(), address: ipv6.to_string()}}

    }
}

pub fn ipv6_to_onion(ipv6:String) -> String {
    let mut step1: Vec<_> = ipv6.split(":").collect();
    let _step2: Vec<_> = step1.drain(0..3).collect();
    let mut step3: String = "".to_string();
    for hextect in step1.iter_mut() {
        if hextect.len() != 4 {
            let good_hextect:String = format!("{:0>4}", hextect);
            step3 = step3 + &good_hextect;
        }
        else {
            step3 = step3 + &hextect.to_string();
        }
    }
    let encode = hex::decode(step3.as_bytes()).unwrap();
    let encode1 = data_encoding::BASE32.encode(&encode).to_lowercase() + ".onion";
    return encode1;
}

pub fn db_folder() -> io::Result<PathBuf> {
    let exe = env::current_exe()?;
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut dir = dir.join("database");
    dir.push("bnetwork.db");
    Ok(dir)
}

pub fn isprivate(addr:[u16; 8]) -> bool {
    let ipv6 = Ipv6Addr::from(addr);
    match ipv6.to_ipv4() {
        Some(ip) => return ip.is_private(),
        None => false,
    }
}

pub fn ip_info(ip:String) -> Result<Value> {
    let sleep_time = time::Duration::from_secs(2);
    let url = "http://ip-api.com/json/".to_string() + &ip;
    let resp = reqwest::blocking::Client::new()
        .get(&url)
        .send().unwrap();
    let data = resp.text().unwrap();
    let v: Value = serde_json::from_str(&data).unwrap();
    thread::sleep(sleep_time);
    println!("{:#?}", v["query"]);
    println!("{:#?}", v["as"]);
    Ok(v)
}