mod bcmessage;
mod bcblocks;
mod bcfile;
mod bcnet;

use clap::{Arg, App};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::process;

use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;

const CHECK_TERMINATION_TIMEOUT:Duration = Duration::from_secs(5);
const THREADS: u64 = 500;

lazy_static! {
    static ref ADRESSES_VISITED: Mutex<HashMap<String, PeerStatus>> = Mutex::new(HashMap::new());
}

const MESSAGE_CHANNEL_SIZE: usize = 100000;
pub static NB_ADDR_TO_TEST: AtomicUsize = AtomicUsize::new(0);

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
    let peer_status = PeerStatus {
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
    // std::mem::drop(address_visited);
    is_waiting
}

pub fn fail(a_peer :String){
    let mut address_status = ADRESSES_VISITED.lock().unwrap();
    address_status.insert(a_peer, peer_status(Status::Failed));
}

pub fn done(a_peer :String) {
    let mut address_status = ADRESSES_VISITED.lock().unwrap();
    address_status.insert(a_peer, peer_status(Status::Done));
}

fn get_connected_peers() -> u64 {
    let mut successful_peer = 0;
    let address_status  = ADRESSES_VISITED.lock().unwrap();
    for (_, peer_status) in address_status.iter(){
        if peer_status.status == Status::Done {
            successful_peer = successful_peer +1;
        }
    }
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

pub fn register_pvm_connection(a_peer:String) {
    let mut address_status = ADRESSES_VISITED.lock().unwrap();
    address_status.insert(a_peer, peer_status(Status::Connected));
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

pub fn check_addr_messages(new_addresses: Vec<String>, address_channel: Sender<String>) -> usize {
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
            bcfile::store_event(&msg);
            address_channel.send(new_peer.to_string()).unwrap();
        }
    }
    new_addresses.len()
}

fn check_pool_size(start_time: SystemTime ){
    loop {
        thread::sleep(CHECK_TERMINATION_TIMEOUT);

        get_peer_status();
        if NB_ADDR_TO_TEST.load(Ordering::Relaxed) < 1 {

            let successful_peers = get_connected_peers();
            let time_spent = SystemTime::now().duration_since(start_time).unwrap_or_default();
            println!("POOL Crawling ends: {:?} new peers in {:?} ", (ADRESSES_VISITED.lock().unwrap()).len(), time_spent);
            println!("{:?} peers successfully connected ", successful_peers);
            process::exit(0);
        }
    }
}

fn main() {

    let start_time: SystemTime = SystemTime::now();
    bcmessage::create_init_message_payload();
    bcfile::load_blocks();
    bcblocks::create_block_message_payload(&bcblocks::BLOCKS_ID.lock().unwrap());

    // eprintln!("{}", hex::encode(bcblocks::get_getblock_message_payload()));
    // eprintln!("{}", hex::encode(bcblocks::get_getheaders_message_payload()));
    // std::process::exit(1);

    // eprintln!("{:?}", known_block);
    // eprintln!("{:?}", bcblocks::BLOCKS_ID.lock().unwrap());
    // std::process::exit(1);

    let (address_channel_sender, address_channel_receiver) = mpsc::channel();
    let (connecting_start_channel_sender, connecting_start_channel_receiver) = chan::sync(MESSAGE_CHANNEL_SIZE);

    let start_adress = parse_args();

    address_channel_sender.send(start_adress).unwrap();
    thread::spawn(move || { check_pool_size(start_time ); });

    for i in 0..THREADS {
        // let counter = Arc::clone(&addresses_to_test);
        let sender = address_channel_sender.clone();
        let recv = connecting_start_channel_receiver.clone();
        thread::spawn(move || { bcnet::handle_one_peer(recv, sender, i);});
    }

    loop {
        let new_peer: String = address_channel_receiver.recv().unwrap();
        connecting_start_channel_sender.send(new_peer);
        NB_ADDR_TO_TEST.fetch_add(1, Ordering::Relaxed);
    }
}
