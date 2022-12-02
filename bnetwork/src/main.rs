use std::path::Path;
use std::{thread, time};

#[path = "database/db.rs"]pub mod db;
#[path = "common/common.rs"]pub mod common;
#[path = "p2p/p2p.rs"]pub mod p2p;


fn main(){
    let sleep_time = time::Duration::from_secs(5);
    let db_path = common::db_folder().unwrap().to_str().unwrap().to_string();
    //print!("{}", db_path);
    let incoming = 0;
    //Para detectar si es la primera ejecución comprobamos que la BBDD (bnetwork.db) existe
    let o = Path::new(&db_path);
    let b = o.exists();
    let mut i = 0; //contador de nodos para limpieza de BBDD
    let mut m = 0; //contador de mantenimiento

    match b {
        //Si existe no hacemos nada continuará al crawler
        true => {}
        //Si no existe creamos la base de datos 
        false => {
            db::create_db().ok();
            db::new_node(&common::first_nodes()).ok();
        }
    };
    loop {
        //Elegimos a un nodo 
        let the_one = db::the_chosen().unwrap();
        if the_one.port() == 0 {
            if m == 0 {
                // Comprobar nodos que aceptaron conexiones entrantes y ahora no
                println!("{} Actualizando nodos que aceptan conexiones entrantes", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string());
                for i in db::update_incoming().unwrap() {
                    if common::scan_port_addr(i) {
                        db::update_open_node(i).ok();
                    }
                }
                m = m + 1;
            }
            if m == 1 {
                // Alimentamos 200 registros con la información de su ip
                println!("{} Actualizando información ip", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string());
                for i in db::ip_info_list().unwrap() {
                    let ip = i.clone();
                    let ip_info = common::ip_info(i).unwrap();
                    db::update_ip_info(ip, 
                        ip_info["country"].to_string().replace("\"", ""), 
                        ip_info["regionName"].to_string().replace("\"", ""), 
                        ip_info["city"].to_string().replace("\"", ""), 
                        ip_info["isp"].to_string().replace("\"", ""), 
                        ip_info["as"].to_string().replace("\"", ""), 
                        ip_info["lat"].to_string().replace("\"", ""),
                        ip_info["lon"].to_string().replace("\"", "")).unwrap();
                }
                m = m + 1;
            }
            if m == 2 {
                // Hacemos un repaso de los nodos para comprobar puertos abiertos
                for i in db::check_incoming().unwrap() {
                    if common::scan_port_addr(i) {
                        db::update_incoming_closed_node(i).ok();
                    }
                }
                m = 0;
                thread::sleep(sleep_time);
            }
        }
        //Tratamos los nodos en TOR (direcciones ONION)
        if the_one.port() == 1 {
            //let the_tor = db::the_chosen_tor().unwrap();
            //println!("{:?}", the_tor);
        } 
        //Si tiene el puerto abierto comenzamos el handshake, en caso contrario actualizamos los datos en la BBDD
        if common::scan_port_addr(the_one) {
            db::update_open_node(the_one).ok();
            println!("{} {:?}", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string(), the_one.to_owned());
            p2p::converse(the_one);
            i = i + 1;
        }
        else {
            db::update_closed_node(incoming, the_one).ok();
            i = i + 1
        }
        println!("{} Vamos {:?} nodos. Cuando lleguemos a mil, limpieza", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string(), i);
        if i == 1000 {
            db::clean_db().ok();
            i = 0;
        }
    }
}
#[test]
fn should_fail() {
 
   unimplemented!();
}