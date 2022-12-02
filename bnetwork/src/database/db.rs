extern crate rusqlite;
extern crate chrono;
use std::net::SocketAddr;
use std::net::IpAddr;

use self::rusqlite::{params, Connection, Result};
use self::rusqlite::NO_PARAMS;
#[path = "../common/common.rs"]pub mod common;
use common::TypeAddress;
//Creación de la BBDD


pub fn create_db() -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;

    conn.execute(
        r#"create table if not exists bnetwork (
             added date,
             detected date,
             scanned date,
             soft text,
             services text,
             type text,
             address text primary key,
             port text,
             incoming integer,
             notes text,
             country text,
             region text,
             city text,
             isp text,
             asn text,
             latitude real,
             longitude real
            )"#,
        NO_PARAMS,
    )?;

    Ok(())
}

//Introducir nodos

pub fn new_node(node: &Vec<SocketAddr>) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    for i in node.iter(){
        let time = chrono::offset::Local::now().to_string();
        if i.is_ipv6(){};
        let addr_type= "ipv4".to_string();
        //La IP es la primary key, si detectamos que existe hacemos update de la fecha detected, si no existe lo creamos.
        conn.execute("INSERT OR IGNORE INTO bnetwork (added, detected, type, address, port) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![time, time, addr_type, i.ip().to_string(), i.port().to_string()],
    )?;
    };
        Ok(())
}

//Devuelve un nodo para analizar
pub fn the_chosen() -> Result<SocketAddr>{
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let mut stmt = conn
        .prepare("SELECT type, address, port
            from bnetwork where (type='ipv4' or type='ipv6') AND (incoming=1 or incoming is NULL) AND (scanned < date('now','-2 days') or scanned is NULL) LIMIT 1")?;
    let mut rows = stmt.query(NO_PARAMS)?;
    let mut address=  String::new();
    let mut port= String::new();
    let mut addr_type=  String::new();
    while let Some(row) = rows.next()? {
        addr_type = row.get(0)?;
        address = row.get(1)?;
        port = row.get(2)?;
    }
    if address.trim().is_empty() {
        let node:SocketAddr = "127.0.0.1:0"
            .parse()
            .expect("Unable to resolve domain");
        Ok(node)
    }
    else if addr_type.contains("onion") {
        let node:SocketAddr = "127.0.0.1:0"
            .parse()
            .expect("Unable to resolve domain");
        Ok(node)
    }
    else{
        let ipaddress = address.parse::<IpAddr>().unwrap();
        let ipport = port.parse::<u16>().unwrap();
        let node:SocketAddr = SocketAddr::new(ipaddress, ipport);
        Ok(node)
    }
}

//Devuelve un nodo TOR
pub fn the_chosen_tor() -> Result<String>{
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let mut stmt = conn
        .prepare("SELECT address
            from bnetwork where type like '%ONION%' AND (incoming=1 or incoming is NULL) AND (scanned < date('now','-2 days') or scanned is NULL) LIMIT 1")?;
    let mut rows = stmt.query(NO_PARAMS)?;
    let mut address=  String::new();
    while let Some(row) = rows.next()? {
        address = row.get(0)?;
    }
    Ok(address)
}
//Actualiza los datos en caso de puerto cerrado
pub fn update_closed_node(connect:i32, node:SocketAddr) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let time = chrono::offset::Local::now().to_string();
    match conn.execute("UPDATE bnetwork SET scanned=?, incoming=? WHERE address=?",
        &[time, connect.to_string(), node.ip().to_string()]) {
            Ok(_updated) => (),
            Err(err) => println!("{} error", err),
        }
        Ok(())
}

pub fn update_open_node(node:SocketAddr) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let connect = 1;
    let conn = Connection::open(db_path)?;
    let time = chrono::offset::Local::now().to_string();
    match conn.execute("UPDATE bnetwork SET scanned=?, incoming=? WHERE address=?",
        &[time, connect.to_string(), node.ip().to_string()]) {
            Ok(_updated) => (),
            Err(err) => println!("{} error", err),
        }
        Ok(())
}

pub fn insert_node(address:TypeAddress, port:String, services:String) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let time = chrono::offset::Local::now().to_string();
    //La IP es la primary key, si detectamos que existe hacemos update de la fecha detected, si no existe lo creamos.
    conn.execute("INSERT OR IGNORE INTO bnetwork (added, detected, type, address, port, services) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params![time, time, address.address_type, address.address, port, services],
    )?;
        Ok(())
}

pub fn update_node(node: SocketAddr, agent:String, services:String) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let time = chrono::offset::Local::now().to_string();
    match conn.execute("UPDATE bnetwork SET scanned=?, soft=?, services=?, incoming=1 WHERE address=?",
        &[time, agent, services, node.ip().to_string()]) {
            Ok(_updated) => (),
            Err(err) => println!("{} error", err),
        }
        Ok(())
}

pub fn addrv2insert(address_type:String, address:String, port:String, services:String) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let time = chrono::offset::Local::now().to_string();
    //La IP es la primary key, si detectamos que existe hacemos update de la fecha detected, si no existe lo creamos.
    conn.execute("INSERT OR IGNORE INTO bnetwork (added, detected, type, address, port, services) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params![time, time, address_type, address, port, services],
    )?;
        Ok(())
}

pub fn addr2vupdate(address:String) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let time = chrono::offset::Local::now().to_string();
    match conn.execute("UPDATE bnetwork SET detected=? WHERE address=?",
        &[time, address]) {
            Ok(_updated) => (),
            Err(err) => println!("{} error", err),
        }
        Ok(())
    }

pub fn update_ip_info(ip: String, country: String, region:String, city:String, isp:String, asn: String, latitude: String, longitude: String) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    match conn.execute("UPDATE bnetwork SET country=?, region=?, city=?, isp=?, asn=?, latitude=?, longitude=? WHERE address=?",
        &[country, region, city, isp, asn, latitude, longitude, ip]) {
            Ok(_updated) => (),
            Err(err) => println!("{} error", err),
        }
        Ok(())
}

pub fn update_detected(node: TypeAddress) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let time = chrono::offset::Local::now().to_string();
    match conn.execute("UPDATE bnetwork SET detected=? WHERE address=?",
        &[time, node.address]) {
            Ok(_updated) => (),
            Err(err) => println!("{} error", err),
        }
        Ok(())
    }

pub fn clean_db() -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    match conn.execute("DELETE FROM bnetwork where detected < date('now','-312 hours')", NO_PARAMS) {
            Ok(_updated) => (),
            Err(err) => println!("{} error", err),
        }
        Ok(())
    }
    
pub fn update_incoming() -> Result<Vec<SocketAddr>>{
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let mut stmt = conn
        .prepare("SELECT type, address, port
            from bnetwork where incoming=0 AND soft is NOT NULL")?;
    let mut rows = stmt.query(NO_PARAMS)?;
    let mut nodes =Vec::new();
    while let Some(row) = rows.next()? {
        let addr_type:String = row.get(0)?;
        let address:String = row.get(1)?;
        let port:String = row.get(2)?;
        if address.trim().is_empty() {
            let node:SocketAddr = "127.0.0.1:0"
                .parse()
                .expect("Unable to resolve domain");
            nodes.push(node);
        }
        else if addr_type.contains("onion") {
            let node:SocketAddr = "127.0.0.1:0"
                .parse()
                .expect("Unable to resolve domain");
            nodes.push(node);
        }
        else{
            let ipaddress = address.parse::<IpAddr>().unwrap();
            let ipport = port.parse::<u16>().unwrap();
            let node:SocketAddr = SocketAddr::new(ipaddress, ipport);
            nodes.push(node);
        }
    }
    Ok(nodes)
}

pub fn ip_info_list() -> Result<Vec<String>>{
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let mut stmt = conn
        .prepare("SELECT address from bnetwork where type like '%ipv%' AND country is NULL LIMIT 200")?;
    let mut rows = stmt.query(NO_PARAMS)?;
    let mut nodes =Vec::new();
    while let Some(row) = rows.next()? {
        let result:String = row.get(0)?;
        nodes.push(result);
    }
    Ok(nodes)
}

pub fn check_incoming() -> Result<Vec<SocketAddr>>{
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let conn = Connection::open(db_path)?;
    let mut stmt = conn
        .prepare("SELECT address || ':' || port AS node
            from bnetwork where type = 'ipv4' AND (incoming=0 or incoming is NULL) ORDER by random() LIMIT 2000")?;
    let mut rows = stmt.query(NO_PARAMS)?;
    let mut nodes =Vec::new();
    while let Some(row) = rows.next()? {
        let result:String = row.get(0)?;
        let node = result
            .parse()
            .expect("Unable to resolve domain");
        nodes.push(node);
    }
    Ok(nodes)
}

pub fn update_incoming_closed_node(node:SocketAddr) -> Result<()> {
    let db_path: String = common::db_folder().unwrap().to_str().unwrap().to_string();
    let connect = 1;
    let conn = Connection::open(db_path)?;
    match conn.execute("UPDATE bnetwork SET incoming=? WHERE address=?",
        &[connect.to_string(), node.ip().to_string()]) {
            Ok(_updated) => (),
            Err(err) => println!("{} error", err),
        }
        Ok(())
}