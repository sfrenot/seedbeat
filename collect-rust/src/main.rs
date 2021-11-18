mod bcmessage;
mod bcblocks;

// extern crate clap;
use clap::{Arg, App};
use std::fs::File;
use std::io::{LineWriter, stdout, Write};
use std::sync::{mpsc, Arc};
use std::thread;
use std::net::{TcpStream};
use crate::bcmessage::{ReadResult, INV, MSG_VERSION, MSG_VERSION_ACK, MSG_GETADDR, CONN_CLOSE, MSG_ADDR, GET_BLOCKS, GET_DATA, BLOCK};
use std::time::{Duration, SystemTime};
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::sync::mpsc::Sender; // Voir si chan::Receiver n'est pas préférable
use chan::{self, Receiver};
use std::process;

const CONNECTION_TIMEOUT:Duration = Duration::from_secs(10);
const CHECK_TERMINATION:Duration = Duration::from_secs(5);
//const NEIGHBOURS: u64 = 1000;
const NEIGHBOURS: u64 = 1000;

lazy_static! {
    static ref ADRESSES_VISITED: Mutex<HashMap<String, PeerStatus>> = Mutex::new(HashMap::new());
    static ref LOGGER: Mutex<LineWriter<Box<dyn Write + Send>>> = Mutex::new(LineWriter::new(Box::new(stdout())));
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

    let arg_file = matches.value_of("file");
    let file: File;
    match arg_file {
        None => panic!("Error parsing file name"),
        Some(f) =>  {
            file = File::create(f).unwrap();
        }
    }

    let mut logger = LOGGER.lock().unwrap();
    *logger = LineWriter::new(Box::new(file));

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

fn store_addr_messages(payload: Vec<u8>, address_channel: Sender<String>) -> usize {
    if payload.len() == 0 {
        return 0;
    }
    let new_addresses = bcmessage::process_addr_message(payload);
    for new_peer in &new_addresses {
        if is_waiting(new_peer.clone()) {
            // new_addr += 1;

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
        let read_result:ReadResult = bcmessage::read_message(&connection);
        // println!("Lecture de {}", target_address);
        // eprint!("R");
        let connection_close = String::from(CONN_CLOSE);
        let get_blocks = String::from(GET_BLOCKS);
        match read_result.error {
            Some(_error) => {
                // eprintln!("Erreur Lecture {}", error);
                in_chain.send(connection_close).unwrap();
                break;
            }
            _ => {
                let command = read_result.command;
                let payload = read_result.payload;

                if command  == String::from(MSG_VERSION) && payload.len() > 0 {
                    let peer = target_address.clone();
                    store_version_message(peer, &payload);
                    in_chain.send(command).unwrap();
                    continue;
                }
                if command == String::from(MSG_VERSION_ACK) {
                    in_chain.send(command).unwrap();
                    continue;
                }
                if command == String::from(MSG_ADDR){
                    //let address_channel = sender.clone();
                    // let num_addr = bcmessage::process_addr_message(payload.clone(), address_channel);

                    if store_addr_messages(payload.clone(),sender.clone()) > ADDRESSES_RECEIVED_THRESHOLD {
                        // in_chain.send(connection_close).unwrap();
                        // in_chain.send(get_blocks).unwrap();

                        match in_chain.send(get_blocks) {
                            Err(error) => {
                                eprintln!("Erreur Send chan : {} ip : {}", error, &target_address);
                                // std::process::exit(1);
                            }
                            _ => {}
                        }
                    }
                }
                if command == String::from(INV){
                    // let vec....
                    // let inv_type:&[u8;1] = &[0x02];
                    // eprintln!("{:?}", payload);
                    let inv_size = payload[0];
                    let inv_length = 36;
                    let block_length = 32;
                    // let mut found = 0;
                    let mut offset = 0;
                    for _i in 0..inv_size {
                        if payload[offset+1] == 0x02 {
                            eprintln!("{:02x?}", payload);

                            // if inv_size > 1 {
                            //     found+=1;
                            // }
                            let mut toto:[u8; 32] = [0x00; 32] ;
                            eprint!("BLOCK ==> ");
                            for val in 0..block_length {
                                toto[val] = payload[offset+inv_length-val];
                                // eprint!("{:02X?}", payload[offset+inv_length-val]);
                            }
                            let block_name = hex::encode(&toto);
                            // eprintln!();
                            if bcblocks::is_new(block_name.clone()) {

                                let get_data = format_args!("{msg}/{block}", msg=GET_DATA, block=block_name).to_string();
                                eprintln!("Recherche du block {}", get_data);

                                match in_chain.send(get_data) {
                                    Err(error) => {
                                        eprintln!("Erreur Send chan : {} ip : {}", error, &target_address);
                                        // std::process::exit(1);
                                    }
                                    _ => {}
                                }
                            }

                        }
                        offset+=inv_length;
                    }
                    // if found > 1 {
                    //     eprintln!("Inventory message block found {:02X?}, {}", payload, payload[1]);
                    //     std::process::exit(1);
                    // }

                    // eprintln!("Inventory message found {:02X?}", payload.clone());
                    // eprintln!("Inv {}", std::str::from_utf8(payload[4..6]));
                    //eprintln!("Inventory message found {}",std::str::from_utf8(&payload).unwrap());
                    // std::process::exit(0);
                }

                if command == String::from(BLOCK){
                    eprintln!("{:?}", payload);
                    std::process::exit(1);
                }

            }
        }
    }
}

fn handle_one_peer(connection_start_channel: Receiver<String>, addresses_to_test : Arc<Mutex<i64>>, address_channel_tx: Sender<String>, _num: u64){

    loop{ //Nodes Management
        // eprintln!(" {} -> attente", num);
        let target_address = connection_start_channel.recv().unwrap();
        // eprintln!("Debut gestion {}", target_address);
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
                        println!("error sending request: {}", e);
                        fail(target_address.clone());
                        break; // From connexion
                    }
                    _ => {}
                }

                // let received_cmd: String;
                // match in_chain_receiver.recv() {
                //     Err(e) => { panic!("Erreur, {}", e);},
                //     Ok(val) => {received_cmd = val;}
                // }
                let received_cmd: String = in_chain_receiver.recv().unwrap();
                if received_cmd != String::from(MSG_VERSION) {
                    // println!("Version Ack not received {}", received_cmd);
                    fail(target_address.clone());
                    break; // From connexion
                }

                match bcmessage::send_request(&connection, MSG_VERSION_ACK) {
                    Err(_) => {
                        println!("error at sending Msg version ack");
                        fail(target_address.clone());
                        break; // From connexion
                    }
                    _ => {}
                }

                let received_cmd = in_chain_receiver.recv().unwrap();
                if received_cmd != String::from(MSG_VERSION_ACK) {
                    println!("Version AckAck not received {}", received_cmd);
                    fail(target_address.clone());
                    break; // From connexion
                }

                match bcmessage::send_request(&connection, MSG_GETADDR) {
                    Err(_) => {
                        println!("error at sending getaddr");
                        fail(target_address.clone());
                        break; // From connexion
                    }
                    _ => {}
                }

                loop { // Handle block Exchanges
                    let received_cmd = in_chain_receiver.recv().unwrap();
                    if received_cmd == String::from(GET_BLOCKS) {
                        eprintln!("==> Envoi GET_BLOCKS {} to: {}", received_cmd, target_address);
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
                    }else if received_cmd == String::from(CONN_CLOSE) {
                        eprintln!("Fermeture {}", &target_address);
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
    let start_time: SystemTime = SystemTime::now();
    bcmessage::create_init_message_payload();
    bcblocks::create_block_message_payload(None);
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
