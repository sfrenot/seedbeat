mod bcmessage;
mod bcblocks;
mod bcfile;

// extern crate clap;
use clap::{Arg, App};
use std::sync::{mpsc, Arc};
use std::thread;
use std::net::{TcpStream};

use std::time::{Duration, SystemTime};
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Mutex;
use std::io::Write;
use lazy_static::lazy_static;
use std::sync::mpsc::Sender; // Voir si chan::Receiver n'est pas préférable
use chan::{self, Receiver};
use std::process;

// use crate::bcmessage::{ReadResult, INV, MSG_VERSION, MSG_VERSION_ACK, MSG_GETADDR, CONN_CLOSE, MSG_ADDR, HEADERS, GET_BLOCKS, BLOCK, GET_DATA};
use crate::bcmessage::{ReadResult, INV, MSG_VERSION, MSG_VERSION_ACK, MSG_GETADDR, CONN_CLOSE, MSG_ADDR, GET_HEADERS, HEADERS, GET_BLOCKS, BLOCK, GET_DATA};
use crate::bcfile::LOGGER;

const CONNECTION_TIMEOUT:Duration = Duration::from_secs(10);
const CHECK_TERMINATION:Duration = Duration::from_secs(5);
//const NEIGHBOURS: u64 = 1000;
const NEIGHBOURS: u64 = 1000;

lazy_static! {
    static ref ADRESSES_VISITED: Mutex<HashMap<String, PeerStatus>> = Mutex::new(HashMap::new());
}

const ADDRESSES_RECEIVED_THRESHOLD: usize = 5;
const MESSAGE_CHANEL_SIZE: usize = 100000;

static mut NB_ADDR: u32 = 0;

#[derive(Debug)]
#[derive(PartialEq)]
enum Status {
    // Waiting,
    Connecting,
    Connected,
    Done,
    Failed
}

#[derive(Debug)]
struct PeerStatus  {
    pub status: Status,
    pub retries: i32,
}

fn generate_peer_status(status: Status, retries: i32) -> PeerStatus{
    let  peer_status = PeerStatus {
        status,
        retries
    };
    return  peer_status;
}

fn peer_status(status: Status) -> PeerStatus{
    return  generate_peer_status(status, 0);
}

fn is_waiting(a_peer: String) -> bool {

    let mut address_visited = ADRESSES_VISITED.lock().unwrap();
    // println!("Before {:?}", address_visited);
    let mut is_waiting = false;
    if !address_visited.contains_key(&a_peer) {
        address_visited.insert(a_peer, peer_status(Status::Connecting));
        is_waiting = true
    }
    // } else {
    //     let peer = address_visited.get(&a_peer).unwrap();
    //     if peer.status == Status::Waiting{
    //         let retries:i32 = peer.retries;
    //         address_visited.insert(a_peer, generate_peer_status(Status::Connecting, retries));
    //         is_waiting = true
    //     }
    // }
    // println!("After {:?}, recherche : {}:{}", address_visited, test, is_waiting);
    std::mem::drop(address_visited);
    is_waiting
}

fn fail(a_peer :String){
    let mut address_status = ADRESSES_VISITED.lock().unwrap();
    address_status.insert(a_peer, peer_status(Status::Failed));
    std::mem::drop(address_status);
}

fn done(a_peer :String) {
    let mut address_status = ADRESSES_VISITED.lock().unwrap();
    address_status.insert(a_peer, peer_status(Status::Done));
    std::mem::drop(address_status);
}

fn get_connected_peers() -> u64 {
    let mut successful_peer = 0;
    let address_status  = ADRESSES_VISITED.lock().unwrap();
    for (_, peer_status) in address_status.iter(){
        if peer_status.status == Status::Done {
            successful_peer = successful_peer +1;
        }
    }
    std::mem::drop(address_status);
    return successful_peer as u64;
}

fn get_peer_status() {
    let mut done = 0;
    let mut fail = 0;
    let mut other = 0;
    let address_status  = ADRESSES_VISITED.lock().unwrap();
    for (_, peer_status) in address_status.iter(){
        if peer_status.status == Status::Done {
            done += 1;
        } else if peer_status.status == Status::Failed {
            fail += 1;
        } else {
            other += 1;
        }
    }
    eprintln!("total: {}, Other: {}, Done: {}, Fail: {}", address_status.len(), other, done, fail);
    std::mem::drop(address_status);
}

fn get_new_peers_size() -> u64 {
    let address_status  = ADRESSES_VISITED.lock().unwrap();
    let size = address_status.len();
    std::mem::drop(address_status);
    return  size as u64;
}

// fn retry_address(a_peer: String)-> bool  {
//     let mut address_status  = ADRESSES_VISITED.lock().unwrap();
//     if address_status[&a_peer].retries > 3  {
//         address_status.insert(a_peer, peer_status(Status::Failed));
//         return false;
//     }
//     //this was different from go code
//     let peer_status =  generate_peer_status(Status::Waiting, address_status[&a_peer].retries + 1);
//     address_status.insert(a_peer,peer_status);
//     std::mem::drop(address_status);
//     return true;
// }

fn register_pvm_connection(a_peer:String) {
    let mut address_status = ADRESSES_VISITED.lock().unwrap();
    address_status.insert(a_peer, peer_status(Status::Connected));
    std::mem::drop(address_status);
}

fn parse_args() -> String {
    let matches = App::new("BC crawl")
        .version("1.0.0")
        .author("Jazmin Ferreiro  <jazminsofiaf@gmail.com>, Stephane Frenot <stephane.frenot@insa-lyon.fr>")
        .arg(Arg::with_name("file")
            .short("-o")
            .long("output")
            .takes_value(true)
            .required(true)
            .help("output file name for crawl"))
        .arg(Arg::with_name("address")
            .short("-s")
            .long("address")
            .takes_value(true)
            .required(true)
            .help(" Initial address for crawling. Format [a.b.c.d]:ppp"))
        .get_matches();

    let arg_address = matches.value_of("address").unwrap_or_else(|| {
        panic!("Error parsing address argument");
        }
    );

    bcfile::open_logfile(matches.value_of("file"));

    String::from(arg_address)
}

fn store_event(msg :&String){
    let mut guard = LOGGER.lock().unwrap();
    guard.write_all(msg.as_ref()).expect("error at logging");
    drop(guard);
}

fn store_version_message(target_address: String, payload: &Vec<u8>){
    //TODO: supprimer le &VEc
    let (_, _, _, _) = bcmessage::process_version_message(payload);
    let mut msg: String  = String::new();
    msg.push_str(format!("Seed: {} \n", target_address).as_ref());
    // msg.push_str(format!("Seed = {}  ", target_address).as_ref());
    // msg.push_str(format!("version = {}   ", version_number).as_str());
    // msg.push_str(format!("user agent = {}   ", user_agent).as_str());
    // msg.push_str(format!("time = {}  ", peer_time.format("%Y-%m-%d %H:%M:%S")).as_str());
    // msg.push_str(format!("now = {}  ", Into::<DateTime<Utc>>::into(SystemTime::now()).format("%Y-%m-%d %H:%M:%S")).as_str());
    // msg.push_str(format!("since = {:?}  ",SystemTime::now().duration_since(SystemTime::from(peer_time)).unwrap_or_default() ).as_str());
    // msg.push_str(format!("services = {:?}\n", services ).as_str());
    store_event(&msg);
    register_pvm_connection(target_address);
}

fn check_addr_messages(new_addresses: Vec<String>, address_channel: Sender<String>) -> usize {
    for new_peer in &new_addresses {
        if is_waiting(new_peer.clone()) {

            let mut msg:String  = String::new();
            msg.push_str(format!("PAR address: {:?}\n", new_peer).as_str());
            // msg.push_str(format!("PAR address: {:?}, ", ip_v4).as_str());
            // msg.push_str(format!("port = {:?}\n", port).as_str());
            // msg.push_str(format!("time = {}  ", date_time.format("%Y-%m-%d %H:%M:%S")).as_str());
            // msg.push_str(format!("now = {}  ", Into::<DateTime<Utc>>::into(SystemTime::now()).format("%Y-%m-%d %H:%M:%S")).as_str());
            // msg.push_str(format!("since = {:?}  ",SystemTime::now().duration_since(SystemTime::from(date_time)).unwrap_or_default() ).as_str());
            // msg.push_str(format!("services = {:?}     ", services ).as_str());
            // msg.push_str(format!("target address = {}\n", target_address ).as_str());

            // println!(" {} -> new peer {} ",target_address, new_peer);
            store_event(&msg);
            address_channel.send(new_peer.to_string()).unwrap();
        }
    }
    new_addresses.len()
}

fn handle_incoming_message(connection:& TcpStream, target_address: String, in_chain: Sender<String>, sender: Sender<String>)  {

    loop {
        // eprintln!("Attente lecture sur {}", target_address);
        let read_result:ReadResult = bcmessage::read_message(&connection);
        // println!("Lecture de {}", target_address);

        match read_result.error {
            Some(_error) => {
                // eprintln!("Erreur Lecture {}: {}", _error, target_address);
                in_chain.send(String::from(CONN_CLOSE)).unwrap();
                break;
            }
            _ => {
                let command = read_result.command;
                let payload = read_result.payload;
                // eprintln!("Command From : {} --> {}", &target_address, &command);
                if command  == String::from(MSG_VERSION) && payload.len() > 0 {
                    let peer = target_address.clone();
                    store_version_message(peer, &payload);
                    // eprintln!("Envoi MSG_VERSION {}", target_address);
                    in_chain.send(command).unwrap();
                    continue;
                }
                if command == String::from(MSG_VERSION_ACK) {
                    // eprintln!("Envoi MSG_VERSION_ACK {}", target_address);
                    in_chain.send(command).unwrap();
                    continue;
                }
                if command == String::from(MSG_ADDR){
                    if check_addr_messages(bcmessage::process_addr_message(&payload), sender.clone()) > ADDRESSES_RECEIVED_THRESHOLD {
                        // eprintln!("GET_BLOCKS {}", target_address);
                        // in_chain.send(String::from(GET_BLOCKS));
                        in_chain.send(String::from(GET_HEADERS)).unwrap();
                    }
                }

                //Testing incoming message
                // if command == String::from(GET_HEADERS){
                //     eprintln!("GET-HEADERS {}", hex::encode(&payload));
                //     std::process::exit(1);
                // }

                if command == String::from(HEADERS){
                    let (idx, block) = bcmessage::process_headers_message(payload);
                    eprintln!("{:?}", bcblocks::BLOCKS_ID.lock().unwrap());
                    eprintln!("{:?}", bcblocks::KNOWN_BLOCK.lock().unwrap());

                    eprintln!("Status : {} -> {}", idx, block);


                    std::process::exit(1);
                }

                // if command == String::from(INV){
                //     //TODO: must migrate to bcmessage::process_inv_message
                //     //TODO: check inv_size -> get_compact_int
                //
                //     let inv_size = payload[0];
                //     let inv_length = 36;
                //     let block_length = 32;
                //     let mut offset = 0;
                //     for _i in 0..inv_size {
                //         if payload[offset+1] == 0x02 {
                //             let mut toto:[u8; 32] = [0x00; 32] ;
                //             // eprint!("BLOCK ==> ");
                //             for val in 0..block_length {
                //                 toto[val] = payload[offset+inv_length-val];
                //             }
                //             if toto[0] != 0x00 {
                //                 eprintln!("Etrange {:02x?}", payload);
                //                 // std::process::exit(1);
                //             } else {
                //                 let block_name = hex::encode(&toto);
                //                 if bcblocks::is_new(block_name.clone()) {
                //
                //                     let get_data = format_args!("{msg}/{block}", msg=GET_DATA, block=block_name).to_string();
                //                     eprintln!("Recherche du block {}", get_data);
                //
                //                     match in_chain.send(get_data) {
                //                         Err(error) => {
                //                             eprintln!("Erreur Send chan : {} ip : {}", error, &target_address);
                //                         }
                //                         _ => {}
                //                     }
                //                 }
                //             }
                //         }
                //         offset+=inv_length;
                //     }
                // }

                // if command == String::from(BLOCK){
                //     //TODO: must migrate to bcmessage
                //     eprintln!("BLOCK : {:02x?}", &payload[..100]);
                //
                //     let hash = sha256d::Hash::hash(&payload[..80]);
                //
                //     let mut previous_block = [0;32];
                //     previous_block.clone_from_slice(&payload[4..36]);
                //     previous_block.reverse();
                //
                //     eprintln!("previous: {}, current: {:?}", hex::encode(&previous_block), hash.to_string());
                //
                //     // !!!!!!!!!!
                //     std::process::exit(1);
                // }

            }
        }
    }
    // eprintln!("Fermeture {}", target_address);
}

fn handle_one_peer(connection_start_channel: Receiver<String>, addresses_to_test : Arc<Mutex<i64>>, address_channel_tx: Sender<String>, _num: u64){

    loop{ //Nodes Management
        let target_address = connection_start_channel.recv().unwrap();
        // eprintln!("Connexion {}, {}", num, target_address);
        let socket: SocketAddr = target_address.parse().unwrap();
        let result = TcpStream::connect_timeout(&socket, CONNECTION_TIMEOUT);
        // eprintln!("Connecté {}, {}", num, target_address);
        if result.is_err() {
            // println!(" {} -> Fail", target_address);
            // println!(" -> Fail to connect {}: {}", target_address, result.err().unwrap());
            fail(target_address.clone());
        } else {
            // println!(" {} -> Success", target_address);
            // println!("Fail to connect {}: {}", target_address, result.err().unwrap());

            let connection = Arc::new(result.unwrap());
            let peer = target_address.clone();

            let (in_chain_sender, in_chain_receiver) = mpsc::channel();

            let connection_clone = connection.clone();
            let sender = address_channel_tx.clone();
            thread::spawn(move || {
                handle_incoming_message(&connection_clone, peer, in_chain_sender, sender);
            });

            loop { //Connection management

                match bcmessage::send_request(&connection, MSG_VERSION) {
                    Err(e) => {
                        eprintln!("Error sending request: {}: {}", e, target_address);
                        fail(target_address.clone());
                        break; // From connexion
                    }
                    _ => {}
                }

                let received_cmd: String = in_chain_receiver.recv().unwrap();
                if received_cmd != String::from(MSG_VERSION) {
                    // eprintln!("Version Ack not received {}, {}", received_cmd, target_address);
                    fail(target_address.clone());
                    break; // From connexion
                }

                match bcmessage::send_request(&connection, MSG_VERSION_ACK) {
                    Err(_) => {
                        eprintln!("error at sending Msg version ack {}", target_address);
                        fail(target_address.clone());
                        break; // From connexion
                    }
                    _ => {}
                }

                let received_cmd = in_chain_receiver.recv().unwrap();
                if received_cmd != String::from(MSG_VERSION_ACK) {
                    eprintln!("Version AckAck not received {}: {}", received_cmd, target_address);
                    fail(target_address.clone());
                    break; // From connexion
                }

                match bcmessage::send_request(&connection, MSG_GETADDR) {
                    Err(_) => {
                        eprintln!("error at sending getaddr: {}", target_address);
                        fail(target_address.clone());
                        break; // From connexion
                    }
                    _ => {}
                }

                loop { // Handle block Exchanges
                    let received_cmd:String = match in_chain_receiver.recv() {
                        Err(e) => {
                            println!("A CORRIGER !!!! : Erreur in Chain {}: {}", e, target_address);
                            std::process::exit(1);
                        },
                        Ok(res) => {res}
                    };
                    // eprintln!("CMD -> {}", received_cmd);

                    if received_cmd == String::from(GET_HEADERS) {
                        // eprintln!("==> Envoi GET_HEADERS {} to: {}", received_cmd, target_address);
                        match bcmessage::send_request(&connection, GET_HEADERS) {
                            Err(_) => {
                                println!("error at sending getHeaders");
                                fail(target_address.clone());
                                break; // From connexion
                            }
                            _ => {}
                        }
                    } else
                    if received_cmd == String::from(GET_BLOCKS) {
                        // eprintln!("==> Envoi GET_BLOCKS {} to: {}", received_cmd, target_address);
                        match bcmessage::send_request(&connection, GET_BLOCKS) {
                            Err(_) => {
                                println!("error at sending getaddr");
                                fail(target_address.clone());
                                break; // From connexion
                            }
                            _ => {}
                        }
                    } else if &received_cmd[..GET_DATA.len()] == String::from(GET_DATA) {
                        eprintln!("Recherche info block {}", &received_cmd[GET_DATA.len()+1..]);
                        match bcmessage::send_request(&connection, &received_cmd) {
                            Err(_) => {
                                println!("error at sending getData");
                                fail(target_address.clone());
                                break; // From connexion
                            }
                            _ => {}
                        }
                    } else if received_cmd == String::from(CONN_CLOSE) {
                        // eprintln!("Fermeture {}", &target_address);
                        done(target_address.clone());
                        break; // From connexion
                    } else {
                        println!("Bad message {}", received_cmd);
                        std::mem::drop(connection);
                        std::process::exit(1);
                    }
                }
                break;
            }
        }
        // eprintln!("Fin gestion {}", target_address);

        let mut guard = addresses_to_test.lock().unwrap();
        // eprintln!("---> {} avant décompte", guard);
        *guard += -1;

        unsafe {
            // eprintln!("---> {} avant décompte addr", NB_ADDR);
            NB_ADDR -= 1;
        }
    }
}

fn check_pool_size(addresses_to_test : Arc<Mutex<i64>>, start_time: SystemTime ){
    // eprint!(".");
    loop {
        thread::sleep(CHECK_TERMINATION);

        let new_peers = get_new_peers_size();
        let nb = *addresses_to_test.lock().unwrap();
        // unsafe {
        //     println!("-> UP {} - {} - add {}", new_peers, nb, NB_ADDR);
        // }
        // eprint!(".");
        // if *addresses_to_test.lock().unwrap() < 1 || new_peers >10000{
        get_peer_status();
        if nb < 1 {

            let successful_peers = get_connected_peers();
            let time_spent = SystemTime::now().duration_since(start_time).unwrap_or_default();
            println!("POOL Crawling ends: {:?} new peers in {:?} ", new_peers, time_spent);
            println!("{:?} peers successfully connected ", successful_peers);
            process::exit(0);
        }
    }
}

fn main() {

    let start_time: SystemTime = SystemTime::now();
    bcmessage::create_init_message_payload();
    let blocks = bcfile::load_blocks();
    let mut idx:usize = 1;
    let mut known_block = bcblocks::KNOWN_BLOCK.lock().unwrap();
    let mut previous: String = "".to_string();

    for item in blocks {
        // eprintln!("-> {}", item.elem);
        known_block.insert(item.elem.clone(), bcblocks::BlockDesc{idx, previous});
        if item.next {
            previous = item.elem.clone();
        } else {
            previous = "".to_string();
        }
        bcblocks::create_block_message_payload(item.elem, item.next);
        idx+=1;
    }

    // eprintln!("{}", hex::encode(bcblocks::get_getblock_message_payload()));
    // eprintln!("{}", hex::encode(bcblocks::get_getheaders_message_payload()));
    // std::process::exit(1);

    eprintln!("{:?}", known_block);
    eprintln!("{:?}", bcblocks::BLOCKS_ID.lock().unwrap());
    // std::process::exit(1);
    drop(known_block);

    let addresses_to_test:Arc<Mutex<i64>> = Arc::new(Mutex::new(0));

    let (address_channel_sender, address_channel_receiver) = mpsc::channel();
    let (connecting_start_channel_sender, connecting_start_channel_receiver) = chan::sync(MESSAGE_CHANEL_SIZE);

    let mut thread_handlers = vec![];
    let start_adress = parse_args();

    address_channel_sender.send(start_adress).unwrap();

    let counter = Arc::clone(&addresses_to_test);
    thread_handlers.push( thread::spawn(move || {
        check_pool_size(counter, start_time );
    }));

    for i in 0..NEIGHBOURS {
        let counter = Arc::clone(&addresses_to_test);
        let sender = address_channel_sender.clone();
        let recv = connecting_start_channel_receiver.clone();
        thread_handlers.push( thread::spawn(move || {
          handle_one_peer(recv, counter, sender, i);
        }));
    }

    loop {
        let new_peer: String = address_channel_receiver.recv().unwrap();
        connecting_start_channel_sender.send(new_peer);

        let mut addresses_to_test = addresses_to_test.lock().unwrap();
        *addresses_to_test += 1;
        unsafe {
            NB_ADDR += 1;
            // println!("n = {}, known peer = {}, addr = {} ", addresses_to_test, get_new_peers_size(), NB_ADDR);
        }
    }
}

// Test blocks checksum V0
// let header = [
//   0x00, 0x00, 0x80, 0x20,
//   0x19, 0x61, 0x47, 0x2a, 0x82, 0x73, 0x1b, 0xeb, 0xe2, 0x40, 0x36, 0x53, 0xed, 0x4f, 0xc7, 0xd8, 0x87, 0x42, 0xd9, 0x4a, 0x69, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
//   0x24, 0x0b, 0x12, 0x0b, 0x83, 0xc7, 0x8c, 0xd4, 0x96, 0x7d, 0x01, 0x77, 0xfc, 0x80, 0x38, 0xeb, 0xaf, 0xba, 0x49, 0xa2, 0xdf, 0x34, 0xbb, 0xbd, 0xba, 0x12, 0x70, 0xbd, 0xfb, 0x22, 0x0b, 0xa1,
//   0x69, 0x08, 0x99, 0x61,
//   0xea, 0x69, 0x0c, 0x17,
//   0x2c, 0x79, 0x25, 0x19,
//   0x22];
//
// eprintln!("{:02x?}", &header[..80]);
// std::process::exit(1);

// let mut hasher = Sha256::new();
// hasher.input(&header);
// let sum = hasher.result();
// let mut hasher2 = Sha256::new();
// hasher2.input(sum);
// let mut result = hasher2.result();
// result.reverse();
// let format = hex::encode(result);
// eprintln!("{:?}", format);

// Test blocks checksum V1
// let hash = sha256d::Hash::hash(&header);
// eprintln!("{:?}", hash.to_string());


// let gen:Vec<u8> = vec![0x00, 0x01, 0x02];
// let gen2 = Vec::from_hex("000102").unwrap();
// // block_message.extend((Vec::from_hex(search_block).unwrap()).to_be_bytes());
// let mut tmp = Vec::hex::from_hex("0000000000000000000c1c499d6f1e87e199633a8c811f18a8e5a86f40b3fb50").unwrap();
// tmp.reverse();
// eprintln!("{:02x?}", tmp);
// std::process::exit(1);

// let toto = hex::encode([0x00, 0x02, 0xFF]);
//
// eprintln!("{:02X?}",gen);
// eprintln!("{:02X?}",gen2);
// dbg!("coucou");
// eprintln!("{}", toto);
// std::process::exit(1);
