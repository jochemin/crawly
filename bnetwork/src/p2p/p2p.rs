extern crate bitcoin;
extern crate rand;
extern crate tor_stream;
extern crate data_encoding;

//#[path = "../common/common.rs"]pub mod common;
#[path = "../database/db.rs"]pub mod db;

use chrono::{offset::TimeZone, Local, NaiveDateTime};
use db::common::TypeAddress;
use db::common::type_address;

use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpStream};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Write;
use std::time::Duration;

use self::rand::Rng;

use self::bitcoin::consensus::{encode};
use self::bitcoin::network::{address, constants, message, message_network, stream_reader::StreamReader};


pub fn converse(address:SocketAddr) {
    let timeout = Duration::from_secs(5);
    let version_message = build_version_message(address);
    
    let first_message = message::RawNetworkMessage {
        magic: constants::Network::Bitcoin.magic(),
        payload: version_message,
    };
    if let Ok(mut stream) = TcpStream::connect_timeout(&address, timeout) {
        stream.set_read_timeout(Some(Duration::new(30,0))).ok();
        let rndnumber = rand::thread_rng().gen::<u64>();
        // Send the message
        let _ = stream.write_all(encode::serialize(&first_message).as_slice());
        //println!("{} Sent version message", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string());
        println!("{} Waiting for addr message", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string());
        // Setup StreamReader
        //let read_stream_no_answer = stream.try_clone().unwrap().read_timeout();
        let read_stream = stream.try_clone().unwrap();
        let mut stream_reader = StreamReader::new(read_stream, None);
        loop {
            // Loop an retrieve new messages
            let reply: message::RawNetworkMessage = match stream_reader.read_next(){
                Ok(reply) => reply,
                _ => break
            };
            //println!("{:?}", reply.payload);
            match reply.payload {
                message::NetworkMessage::Version(x) => {
                    let soft = x.user_agent;
                    let services = x.services.to_string();
                    db::update_node(address, soft, services).ok();
                    
                    let sendaddrv2_message = message::RawNetworkMessage{
                        magic: constants::Network::Bitcoin.magic(),
                        payload: message::NetworkMessage::SendAddrV2
                    };
                    let _ = stream.write_all(encode::serialize(&sendaddrv2_message).as_slice());

                    let second_message = message::RawNetworkMessage {
                        magic: constants::Network::Bitcoin.magic(),
                        payload: message::NetworkMessage::Verack,
                    };

                    let _ = stream.write_all(encode::serialize(&second_message).as_slice());
                    //println!("Sent verack message");
                }
                message::NetworkMessage::Verack => {
                    //println!("Received verack message: {:?}", reply.payload);
                }
                message::NetworkMessage::Ping(_) => {
                    //println!("Received Ping message: {:?}", reply.payload);
                    
                    let third_message = message::RawNetworkMessage {
                        magic: constants::Network::Bitcoin.magic(),
                        payload: message::NetworkMessage::Pong(rndnumber),
                    };

                    let _ = stream.write_all(encode::serialize(&third_message).as_slice());
                    //println!("Sent Pong message");

                    let fourth_message = message::RawNetworkMessage {
                        magic: constants::Network::Bitcoin.magic(),
                        payload: message::NetworkMessage::GetAddr,
                    };

                    let _ = stream.write_all(encode::serialize(&fourth_message).as_slice());
                    //println!("Sent Getaddr message");
                }
                message::NetworkMessage::Addr(ref x) => {
                    //println!("Received Addr message: {:?}", reply.payload);
                    //println!("{:?} {:?}", {}, x.len());
                    let mut cont = 1;
                    if x.len() == 1 {}
                    else {
                        let time = chrono::offset::Local::now() - chrono::Duration::days(14);  // Tratamos las direcciones con un timestamp menor a 14 días
                        println!("{} Addr message received", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string());
                        for addr in x {
                            if db::common::isprivate(addr.1.address) {}
                                //println!("IP privada")}
                            else{
                                cont = cont + 1;
                                //println!("Addr message: {:?}", Local.from_local_datetime(&NaiveDateTime::from_timestamp(addr.0.into(), 0)).unwrap());
                                if Local.from_local_datetime(&NaiveDateTime::from_timestamp(addr.0.into(), 0)).unwrap() > time {
                                    let address:TypeAddress = type_address(addr.1.address);
                                    let node:TypeAddress = type_address(addr.1.address);
                                    if addr.1.services.to_string().contains("0x"){}
                                    else{
                                        db::update_detected(node).ok();
                                        db::insert_node(address, addr.1.port.to_string(), addr.1.services.to_string()).ok();
                                    }
                                }  
                            }
                        }
                        println!("{} {} addresses processed.", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string(), x.len());
                        break;
                    }
                }
                message::NetworkMessage::AddrV2(ref x) => {
                    let mut cont = 1;
                    if x.len() == 1 {}
                    else {
                        let time = chrono::offset::Local::now() - chrono::Duration::days(14);  // Tratamos las direcciones con un timestamp menor a 14 días
                        println!("{} AddrV2 message received", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string());
                        for entry in x {
                            if Local.from_local_datetime(&NaiveDateTime::from_timestamp(entry.time.into(), 0)).unwrap() > time {
                                match entry.addr {
                                    bitcoin::network::address::AddrV2::Ipv4(ref addr) => {
                                        if addr.is_private() {}
                                        else{
                                            cont = cont + 1;
                                            let address = &addr.to_string();
                                            let addr_type = "ipv4";
                                            let port = &entry.port.to_string();
                                            //let services = ServiceFlags::has(entry.services, ServiceFlags::NETWORK);
                                            let services = format!("{}", entry.services);
                                            db::addr2vupdate(address.to_string()).ok();
                                            db::addrv2insert(addr_type.to_string(), address.to_string(), port.to_string(), services).ok();
                                        }
                                    },
                                    bitcoin::network::address::AddrV2::Ipv6(ref addr) => {
                                        cont = cont + 1;
                                        let address = &addr.to_string();
                                        let addr_type = "ipv6";
                                        let port = &entry.port.to_string();
                                        let services_up = format!("{}", entry.services);
                                        let services_new = services_up.clone();
                                        db::addr2vupdate(address.to_string()).ok();
                                        db::addrv2insert(addr_type.to_string(), address.to_string(), port.to_string(), services_new).ok();
                                    },
                                    bitcoin::network::address::AddrV2::TorV2(ref bytes) =>  {
                                        cont = cont + 1;
                                        let address = data_encoding::BASE32.encode(bytes).to_lowercase() + ".onion";
                                        let addr_type = "onionv2";
                                        let port = &entry.port.to_string();
                                        let services = format!("{}", entry.services);
                                        db::addr2vupdate(address.to_string()).ok();
                                        db::addrv2insert(addr_type.to_string(), address.to_string(), port.to_string(), services).ok();
                                    },
                                    bitcoin::network::address::AddrV2::TorV3(ref bytes) => {
                                        cont = cont + 1;
                                        let address = data_encoding::BASE32.encode(bytes).to_lowercase() + ".onion";
                                        let addr_type = "onionv3";
                                        let port = &entry.port.to_string();
                                        let services = format!("{}", entry.services);
                                        db::addr2vupdate(address.to_string()).ok();
                                        db::addrv2insert(addr_type.to_string(), address.to_string(), port.to_string(), services).ok();
                                    },
                                    bitcoin::network::address::AddrV2::I2p(ref bytes) => {
                                        cont = cont + 1;
                                        let address = data_encoding::BASE32.encode(bytes).to_lowercase();
                                        let addr_type = "i2p";
                                        let port = &entry.port.to_string();
                                        let services = format!("{}", entry.services);
                                        db::addr2vupdate(address.to_string()).ok();
                                        db::addrv2insert(addr_type.to_string(), address.to_string(), port.to_string(), services).ok();
                                    },
                                    bitcoin::network::address::AddrV2::Cjdns(ref addr) => {
                                        cont = cont + 1;
                                        let address = &addr.to_string();
                                        let addr_type = "cjdns";
                                        let port = &entry.port.to_string();
                                        let services = format!("{}", entry.services);
                                        db::addr2vupdate(address.to_string()).ok();
                                        db::addrv2insert(addr_type.to_string(), address.to_string(), port.to_string(), services).ok();
                                    },
                                    bitcoin::network::address::AddrV2::Unknown(_network, ref _bytes) => {},
                                }
                            }
                    //    let address:TypeAddress = TypeAddress(addrv2.1.address);
                    //    db::insert_node(address, addrv2.1.port.to_string(), addr.1.services.to_string());
                        }
                    println!("{} {} addresses processed.", chrono::offset::Local::now().format("%d-%m-%Y %H:%M").to_string(), x.len());   
                    break;
                    }
                }
                _ => {
                    //println!("Received unknown message: {:?}", reply);
                    //break;  <- quito este break porque no paramos hasta recibir el mensaje addr
                }
            }
        }
        let _ = stream.shutdown(Shutdown::Both);
    } else {
        eprintln!("Failed to open connection");
    }
}

fn build_version_message(address: SocketAddr) -> message::NetworkMessage {
    // Building version message, see https://en.bitcoin.it/wiki/Protocol_documentation#version
    let my_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);

    // "bitfield of features to be enabled for this connection"
    let services = constants::ServiceFlags::NONE;

    // "standard UNIX timestamp in seconds"
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time error")
        .as_secs();

    // "The network address of the node receiving this message"
    let addr_recv = address::Address::new(&address, constants::ServiceFlags::NONE);

    // "The network address of the node emitting this message"
    let addr_from = address::Address::new(&my_address, constants::ServiceFlags::NONE);

    // "Node random nonce, randomly generated every time a version packet is sent. This nonce is used to detect connections to self."
    let nonce = rand::thread_rng().gen::<u64>();

    // "User Agent (0x00 if string is 0 bytes long)"
    let user_agent = String::from("Crawly");

    // "The last block received by the emitting node"
    let start_height: i32 = 0;

    // Construct the message
    message::NetworkMessage::Version(message_network::VersionMessage::new(
        services,
        timestamp as i64,
        addr_recv,
        addr_from,
        nonce,
        user_agent,
        start_height,
    ))
}