mod bcmessage;

// extern crate clap;
use chrono::{DateTime, NaiveDateTime, Utc};
use clap::{Arg, App};
use std::fs::File;
use std::io::{LineWriter, stdout, Write, Cursor};
use std::sync::{mpsc, Arc};
use std::thread;
use std::net::{TcpStream, IpAddr};
use crate::bcmessage::{ReadResult, INV, MSG_VERSION, MSG_VERSION_ACK, MSG_GETADDR, CONN_CLOSE, MSG_ADDR, GET_BLOCKS };
use std::time::{Duration, SystemTime};
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;
use byteorder::{ReadBytesExt, LittleEndian, BigEndian};
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

// storage length
const UNIT_16: u8 = 0xFD;
const UNIT_32: u8 = 0xFE;
const UNIT_64: u8 = 0xFF;

const SIZE_FD: u64 = 0xFD;
const SIZE_FFFF: u64 = 0xFFFF;
const SIZE_FFFF_FFFF: u64 = 0xFFFFFFFF;

const STORAGE_BYTE :usize= 0;
const NUM_START :usize= 1;
//const UNIT_8_END: usize = 2;
const UNIT_16_END: usize = 3;
const UNIT_32_END: usize = 5;
const UNIT_64_END: usize = 9;

// offset for version cmd
const VERSION_END: usize =4;
const TIMESTAMP_END:usize= 20;
const USER_AGENT: usize = 80;

// offset for addr cmd
const ADDRESS_LEN: usize =30;
const TIME_FIELD_END: usize =4;
const SERVICES_END:usize = 12;
const IP_FIELD_END:usize= 28;
const PORT_FIELD_END:usize= 30;

const ADDRESSES_RECEIVED_THRESHOLD: u64 = 5;
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

fn get_compact_int(payload: &Vec<u8>) -> u64 {
    let storage_length: u8 = payload[STORAGE_BYTE];

    if storage_length == UNIT_16 {
        let mut bytes_reader = Cursor::new(payload[NUM_START..UNIT_16_END].to_vec());
        return bytes_reader.read_u16::<LittleEndian>().unwrap() as u64;
    }
    if storage_length == UNIT_32 {
        let mut bytes_reader = Cursor::new(payload[NUM_START..UNIT_32_END].to_vec());
        return bytes_reader.read_u32::<LittleEndian>().unwrap() as u64;
    }
    if storage_length == UNIT_64 {
        let mut bytes_reader = Cursor::new(payload[NUM_START..UNIT_64_END].to_vec());
        return bytes_reader.read_u64::<LittleEndian>().unwrap() as u64;
    }
    return storage_length as u64;

}

fn get_start_byte(variable_length_int: & usize) -> usize {
    let size = *variable_length_int as u64;
    if size < SIZE_FD {
        return NUM_START;
    }
    if size <= SIZE_FFFF {
        return  UNIT_16_END;
    }
    if size <= SIZE_FFFF_FFFF {
        return  UNIT_32_END
    }
    return UNIT_64_END
}

fn get_date_time(time_vec: Vec<u8>) -> DateTime<Utc>{
    let mut time_field =  Cursor::new(time_vec);
    let time_int = time_field.read_u32::<LittleEndian>().unwrap() as i64;
    return DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(time_int, 0), Utc);
}

fn process_version_message( target_address: String, payload: &Vec<u8>){

    let mut version_field =  Cursor::new(payload[..VERSION_END].to_vec());
    let _version_number = version_field.read_u32::<LittleEndian>().unwrap() as i64;
    let _services = payload[VERSION_END..SERVICES_END].to_vec();
    let _peer_time = get_date_time(payload[SERVICES_END..TIMESTAMP_END].to_vec());

    let useragent_size = get_compact_int(&payload[USER_AGENT..].to_vec()) as usize;
    let start_byte= get_start_byte(&useragent_size);

    let mut user_agent = String::new();
    if USER_AGENT + start_byte + useragent_size < payload.len() {
        if useragent_size > 0 {
            let user_agent_slice = &payload[(USER_AGENT + start_byte)..(USER_AGENT + start_byte + useragent_size)];
            user_agent.push_str(String::from_utf8(user_agent_slice.to_vec()).unwrap().as_str() );

        }
    }

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

fn read_addresses(payload: Vec<u8>, address_channel: Sender<String>, addr_number: u64){

    let start_byte = get_start_byte(& (addr_number as usize));
    let mut read_addr = 0 ;
    // let mut new_addr = 0;
    while read_addr < addr_number {

        let addr_begins_at = start_byte + (ADDRESS_LEN * read_addr as usize);
        let _date_time = get_date_time(payload[addr_begins_at..addr_begins_at+ TIME_FIELD_END].to_vec());
        let _services = payload[addr_begins_at+ TIME_FIELD_END..addr_begins_at+ SERVICES_END].to_vec();
        let ip_addr_field = payload[addr_begins_at+ SERVICES_END..addr_begins_at+ IP_FIELD_END].to_vec();

        let mut array_v6 = [0; 16];
        array_v6.copy_from_slice(&ip_addr_field[..]);
        let _ip_v6 = IpAddr::from(array_v6);

        let mut array_v4 = [0; 4];
        array_v4.copy_from_slice(&ip_addr_field[12..]);
        let ip_v4 = IpAddr::from(array_v4);

        let mut port_field = Cursor::new(payload[addr_begins_at+ IP_FIELD_END..addr_begins_at+ PORT_FIELD_END].to_vec());
        let port = port_field.read_u16::<BigEndian>().unwrap();

        let new_peer : String = format!("{}:{:?}", ip_v4, port);
        if is_waiting(new_peer.clone()){
            // new_addr += 1;

            let mut msg:String  = String::new();
            msg.push_str(format!("PAR address: {:?}, ", ip_v4).as_str());
            msg.push_str(format!("port = {:?}\n", port).as_str());
            // msg.push_str(format!("time = {}  ", date_time.format("%Y-%m-%d %H:%M:%S")).as_str());
            // msg.push_str(format!("now = {}  ", Into::<DateTime<Utc>>::into(SystemTime::now()).format("%Y-%m-%d %H:%M:%S")).as_str());
            // msg.push_str(format!("since = {:?}  ",SystemTime::now().duration_since(SystemTime::from(date_time)).unwrap_or_default() ).as_str());
            // msg.push_str(format!("services = {:?}     ", services ).as_str());
            // msg.push_str(format!("target address = {}\n", target_address ).as_str());

            // println!(" {} -> new peer {} ",target_address, new_peer);
            store_event(&msg);
            address_channel.send(new_peer).unwrap();
        }
        read_addr = read_addr +1;
    }
    // eprintln!("--> Ajout {} noeuds", new_addr);
}

fn process_addr_message(payload: Vec<u8> , address_channel: Sender<String>) -> u64{
    if payload.len() == 0 {
        return 0;
    }
    let addr_number = get_compact_int(&payload);
    if addr_number < 2 {
        return addr_number;
    }
    read_addresses(payload, address_channel, addr_number);

    return addr_number;
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
                    process_version_message(peer, &payload);
                    in_chain.send(command).unwrap();
                    continue;
                }
                if command == String::from(MSG_VERSION_ACK) {
                    in_chain.send(command).unwrap();
                    continue;
                }
                if command == String::from(MSG_ADDR){
                    let address_channel = sender.clone();
                    let num_addr = process_addr_message(payload.clone(), address_channel);
                    if num_addr > ADDRESSES_RECEIVED_THRESHOLD {
                        // in_chain.send(connection_close).unwrap();
                        in_chain.send(get_blocks).unwrap();
                    }
                }
                if command == String::from(INV){
                    // let vec....
                    // let inv_type:&[u8;1] = &[0x02];
                    let inv_size = payload[0];
                    let inv_length = 36;
                    let block_length = 32;
                    let mut found = false;
                    let mut offset = 0;
                    for _i in 0..inv_size {
                        if payload[offset+1] == 0x02 {
                            if inv_size > 1 {
                                found = true;
                            }
                            let mut toto:[u8; 32] = [0x00; 32] ;
                            eprint!("BLOCK ==> ");
                            for val in 0..block_length {
                                toto[val] = payload[offset+inv_length-val];
                                eprint!("{:02X?}", payload[offset+inv_length-val]);
                            }
                            eprintln!();
                            eprintln!("Result {}", hex::encode(toto));
                        }
                        offset+=inv_length;
                    }
                    if found {
                        eprintln!("Inventory message found {:02X?}, {}", payload, payload[1]);
                        std::process::exit(1);
                    }

                    // eprintln!("Inventory message found {:02X?}", payload.clone());
                    // eprintln!("Inv {}", std::str::from_utf8(payload[4..6]));
                    //eprintln!("Inventory message found {}",std::str::from_utf8(&payload).unwrap());
                    // process::exit(0);
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
                    },
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
                    },
                    _ => {}
                }

                let received_cmd = in_chain_receiver.recv().unwrap();
                if received_cmd == String::from(GET_BLOCKS) {
                    eprintln!("==> Envoi GET_BLOCKS {}", received_cmd);
                    match bcmessage::send_request(&connection, GET_BLOCKS) {
                        Err(_) => {
                            println!("error at sending getaddr");
                            fail(target_address.clone());
                            break; // From connexion
                        },
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
    // let toto = hex::encode([0x00, 0x02, 0xFF]);
    //
    // eprintln!("{:02X?}",gen);
    // eprintln!("{:02X?}",gen2);
    // dbg!("coucou");
    // eprintln!("{}", toto);
    // std::process::exit(1);
    let start_time: SystemTime = SystemTime::now();
    bcmessage::init();
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
