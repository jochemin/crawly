extern crate spin_sleep;
extern crate rusqlite;
extern crate chrono;

use std::path::{Path, PathBuf};
use std::time::Duration;
use std::env;
use chrono::prelude::*;
use self::rusqlite::{params, NO_PARAMS, Connection, Result, OpenFlags};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    //ubicación de la bbdd histórico
    let exe = env::current_exe()?;
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut dir = dir.join("database");
    dir.push("crawly_history.db");

    let b = Path::new(&dir).exists();
    match b {
        true => {println!("Database detected")}
        false => {create_db(dir).ok();}
    }

    loop{
        // We delete information from geo, clients
        let exe = env::current_exe().unwrap();
        let dir = exe.parent().expect("Executable must be in some directory");
        let mut dir = dir.join("database");
        dir.push("crawly_history.db");
        let conn = Connection::open(dir)?;
        let delete1 = "delete from geo";
        let delete2 = "delete from clients";
        let delete3 = "delete from nodes_by_country";
        let delete4 = "delete from nodes_by_city";
        conn.execute(delete1, NO_PARAMS)?;
        conn.execute(delete2, NO_PARAMS)?;
        conn.execute(delete3, NO_PARAMS)?;
        conn.execute(delete4, NO_PARAMS)?;
        conn.execute_batch("pragma journal_mode=WAL").unwrap();
        let conn = Connection::open_with_flags("./database/bnetwork.db", OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        //conn.execute_batch("pragma journal_mode=WAL").unwrap(); //para minimizar errores de "database locked"

        // Incoming nodes
        let mut stmt = conn.prepare("SELECT count (address) FROM bnetwork where incoming = 1 AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut incoming_nodes=  0;
        while let Some(row) = rows.next()? {
           incoming_nodes = row.get(0)?;
        }
        update_db_table("incoming_nodes".to_string(), get_time(), incoming_nodes).ok();

        // Total nodes
        let mut stmt = conn.prepare("SELECT count(address),
                                    (SELECT count(incoming) FROM bnetwork
                                    where detected > datetime('now', '-13 days') AND incoming = 1) AS incoming
                                    FROM bnetwork  where detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut total_nodes=  0;
        let mut incoming = 0;
        while let Some(row) = rows.next()? {
           total_nodes = row.get(0)?;
           incoming = row.get(1)?;
        }
        update_db_table4("total_nodes".to_string(), get_time(), total_nodes, incoming).ok();

        // Private nodes
        let mut stmt = conn.prepare("SELECT count (address) FROM bnetwork where incoming = 0 or incoming is NULL AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut private_nodes=  0;
        while let Some(row) = rows.next()? {
           private_nodes = row.get(0)?;
        }
        update_db_table("private_nodes".to_string(), get_time(), private_nodes).ok();

        // Archive nodes
        let mut stmt = conn.prepare("SELECT count (address) FROM bnetwork where services like '%NETWORK|%' AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut archive_nodes=  0;
        while let Some(row) = rows.next()? {
           archive_nodes = row.get(0)?;
        }
        update_db_table("archive_nodes".to_string(), get_time(), archive_nodes).ok();

        // Pruned nodes
        let mut stmt = conn.prepare("SELECT count (address) FROM bnetwork where services not like '%NETWORK|%' AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut pruned_nodes=  0;
        while let Some(row) = rows.next()? {
           pruned_nodes = row.get(0)?;
        }
        update_db_table("pruned_nodes".to_string(), get_time(), pruned_nodes).ok();

        // Segwit nodes
        let mut stmt = conn.prepare("SELECT count (address) FROM bnetwork where services like '%WITNESS|%' AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut segwit_nodes=  0;
        while let Some(row) = rows.next()? {
           segwit_nodes = row.get(0)?;
        }
        update_db_table("segwit_nodes".to_string(), get_time(), segwit_nodes).ok();

        // Block filter nodes (neutrino)
        let mut stmt = conn.prepare("SELECT count (address) FROM bnetwork where services like '%COMPACT_FILTERS%' AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut block_filter_nodes=  0;
        while let Some(row) = rows.next()? {
            block_filter_nodes = row.get(0)?;
        }
        update_db_table("block_filter_nodes".to_string(), get_time(), block_filter_nodes).ok();

        // Bloom filter nodes (BIP037)
        let mut stmt = conn.prepare("SELECT count (address) FROM bnetwork where services like '%BLOOM%' AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut bloom_filter_nodes=  0;
        while let Some(row) = rows.next()? {
            bloom_filter_nodes = row.get(0)?;
        }
        update_db_table("bloom_filter_nodes".to_string(), get_time(), bloom_filter_nodes).ok();

        // ipv4 nodes
        let mut stmt = conn.prepare("SELECT count(address),
                                    (SELECT count(incoming) FROM bnetwork
                                    where type ='ipv4' AND detected > datetime('now', '-13 days') AND incoming = 1) AS incoming
                                    FROM bnetwork  where type ='ipv4' AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut ipv4_nodes=  0;
        let mut ipv4_incoming= 0;
        while let Some(row) = rows.next()? {
            ipv4_nodes = row.get(0)?;
            ipv4_incoming = row.get(1)?;
        }
        update_db_table4("ipv4_nodes".to_string(), get_time(), ipv4_nodes, ipv4_incoming).ok();

        // ipv6 nodes
        let mut stmt = conn.prepare("SELECT count(address),
                                    (SELECT count(incoming) FROM bnetwork
                                    where type ='ipv6' AND detected > datetime('now', '-13 days') AND incoming = 1) AS incoming
                                    FROM bnetwork  where type ='ipv6' AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut ipv6_nodes=  0;
        let mut ipv6_incoming = 0;
        while let Some(row) = rows.next()? {
            ipv6_nodes = row.get(0)?;
            ipv6_incoming = row.get(1)?;
        }
        update_db_table4("ipv6_nodes".to_string(), get_time(), ipv6_nodes, ipv6_incoming).ok();

        // onion nodes
        let mut stmt = conn.prepare("SELECT count(type) AS onionv2,
                                    (SELECT count(type) FROM bnetwork
                                    where type ='onionv3' AND detected > datetime('now', '-13 days')) AS onionv3
                                    FROM bnetwork where type ='onionV2' AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut onionv2_nodes=  0;
        let mut onionv3_nodes=  0;
        while let Some(row) = rows.next()? {
            onionv2_nodes = row.get(0)?;
            onionv3_nodes = row.get(1)?;
        }
        update_onion_table("onion_nodes".to_string(), get_time(), onionv2_nodes, onionv3_nodes).ok();

        // Location
        let mut stmt = conn.prepare("select country, region, city, isp, asn from bnetwork where country is not NULL AND detected > datetime('now', '-13 days')")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        while let Some(row) = rows.next()? {
            let country = row.get(0)?;
            let region = row.get(1)?;
            let city = row.get(2)?;
            let isp = row.get(3)?;
            let asn = row.get(4)?;
        //    let latitude = row.get(5)?;
        //    let longitude = row.get(6)?;
            update_geo("geo".to_string(),country, region, city, isp, asn)?;
        }

        // Clients
        let mut stmt = conn.prepare("select count(address), soft from bnetwork where soft is not NULL AND detected > datetime('now', '-13 days') GROUP by soft ORDER BY COUNT(address) DESC")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        while let Some(row) = rows.next()? {
            let number = row.get(0)?;
            let soft = row.get(1)?;
            update_clients("clients".to_string(), soft, number)?;
        }

        // Nodes by city
        let mut stmt = conn.prepare("select count(address), city from bnetwork where city is not NULL AND detected > datetime('now', '-13 days') GROUP by city ORDER BY COUNT(address) DESC")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        while let Some(row) = rows.next()? {
            let city_nodes = row.get(0)?;
            let city_name = row.get(1)?;
            update_db_table2("nodes_by_city".to_string(), get_time(), city_name, city_nodes)?;
        }
            
        // Nodes by country
        let mut stmt = conn.prepare("select count(address), country from bnetwork where country is not NULL AND detected > datetime('now', '-13 days') GROUP by country ORDER BY COUNT(address) DESC")?;
        let mut rows = stmt.query(NO_PARAMS)?;
        while let Some(row) = rows.next()? {
            let country_nodes = row.get(0)?;
            let country_name = row.get(1)?;
            update_db_table3("nodes_by_country".to_string(), get_time(), country_name, country_nodes)?;
        }
        println!("Sleeping");
        spin_sleep::sleep(get_duration());
    }
}

fn create_db(dir:PathBuf) -> Result<()> {
    let conn = Connection::open(dir)?;
    
    conn.execute(r#"create table if not exists total_nodes (date date, number integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists incoming_nodes (date date, number integer)"#, NO_PARAMS,)?;
    conn.execute(r#"create table if not exists private_nodes (date date, number integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists segwit_nodes (date date, number integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists block_filter_nodes (date date, number integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists bloom_filter_nodes (date date, number integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists archive_nodes (date date, number integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists pruned_nodes (date date, number integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists ipv4_nodes (date date, number integer, incoming integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists ipv6_nodes (date date, number integer, incoming integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists onion_nodes (date date, onionv2 integer, onionv3 integer, incoming integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists nodes_by_country (date date, country text, number integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists nodes_by_city (date date, city text, number integer)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists geo (country text, region text, city text, isp text, asn text, longitude float, latitude float)"#,NO_PARAMS,)?;
    conn.execute(r#"create table if not exists clients (client text, number integer)"#,NO_PARAMS,)?;
    conn.execute_batch("pragma journal_mode=WAL").unwrap();

    Ok(())
}


fn update_db_table(table:String, date: String, number: i32) -> Result<()> {
    let exe = env::current_exe().unwrap();
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut dir = dir.join("database");
    dir.push("crawly_history.db");
    let conn = Connection::open(dir)?;
    let sql_string = "Insert into ".to_owned() + &table + "(date, number) VALUES (?1, ?2)";
    conn.execute(&sql_string, params![date, number],)?;
    
    Ok(())
}

fn update_onion_table(table:String, date: String, onionv2: i32, onionv3: i32) -> Result<()> {
    let exe = env::current_exe().unwrap();
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut dir = dir.join("database");
    dir.push("crawly_history.db");
    let conn = Connection::open(dir)?;
    let sql_string = "Insert into ".to_owned() + &table + "(date, onionv2, onionv3) VALUES (?1, ?2, ?3)";
    conn.execute(&sql_string, params![date, onionv2, onionv3],)?;
    
    Ok(())
}

fn update_db_table2(table:String, date: String, value: String, number: i32) -> Result<()> {
    let exe = env::current_exe().unwrap();
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut dir = dir.join("database");
    dir.push("crawly_history.db");
    let conn = Connection::open(dir)?;
    let sql_string = "Insert into ".to_owned() + &table + "(date, city, number) VALUES (?1, ?2, ?3)";
    conn.execute(&sql_string, params![date, value, number],)?;
    
    Ok(())
}

fn update_db_table3(table:String, date: String, value: String, number: i32) -> Result<()> {
    let exe = env::current_exe().unwrap();
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut dir = dir.join("database");
    dir.push("crawly_history.db");
    let conn = Connection::open(dir)?;
    let sql_string = "Insert into ".to_owned() + &table + "(date, country, number) VALUES (?1, ?2, ?3)";
    conn.execute(&sql_string, params![date, value, number],)?;
    
    Ok(())
}

fn update_db_table4(table:String, date: String, number: i32, incoming:i32) -> Result<()> {
    let exe = env::current_exe().unwrap();
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut dir = dir.join("database");
    dir.push("crawly_history.db");
    let conn = Connection::open(dir)?;
    let sql_string = "Insert into ".to_owned() + &table + "(date, number, incoming) VALUES (?1, ?2, ?3)";
    conn.execute(&sql_string, params![date, number, incoming],)?;
    
    Ok(())
}

fn update_geo(table: String, country: String, region: String, city: String, isp: String, asn:String) -> Result<()> {
    
//    println!("{:#?}", latitude);
    let exe = env::current_exe().unwrap();
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut dir = dir.join("database");
    dir.push("crawly_history.db");
    let conn = Connection::open(dir)?;
    let sql_string = "Insert into ".to_owned() + &table + "(country, region, city, isp, asn) VALUES (?1, ?2, ?3, ?4, ?5)";
    conn.execute(&sql_string, params![country, region, city, isp, asn],)?;
    
    Ok(())
}

fn update_clients(table:String, value: String, number: i32) -> Result<()> {
    let exe = env::current_exe().unwrap();
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut dir = dir.join("database");
    dir.push("crawly_history.db");
    let conn = Connection::open(dir)?;
    let sql_string = "Insert into ".to_owned() + &table + "(client, number) VALUES (?1, ?2)";
    conn.execute(&sql_string, params![value, number],)?;
    
    Ok(())
}

fn get_time() -> String {
    let utc:DateTime<Utc> = Utc::now().round_subsecs(0);
    let time = utc.day().to_string() + "/" + &utc.month().to_string() + "/" + &utc.year().to_string() + " " + &utc.hour().to_string() + ":00";
    return time;
}

fn get_duration() -> Duration {
    let now = Utc::now().naive_utc();
    let add_hour = now + chrono::Duration::hours(1);
    let next_hour_string = add_hour.round_subsecs(0).format("%Y-%m-%d %H:10:00").to_string();
    let next_hour = NaiveDateTime::parse_from_str(&next_hour_string, "%Y-%m-%d %H:%M:%S").ok().unwrap();
    //let next_hour = NaiveDateTime::parse_from_str(&add_hour.format("%Y-%m-%dT%H:%M:%s").to_string(), "%Y-%m-%dT%H:%M:%s").ok();
    let seconds = next_hour.signed_duration_since(now);
    return Duration::from_secs(seconds.num_seconds() as u64);
}

    #[test]
fn should_fail() {
 
   unimplemented!();
}