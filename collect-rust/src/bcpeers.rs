use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::Sender;

use crate::bcfile as bcfile;
use lazy_static::lazy_static;

lazy_static! {
    static ref ADRESSES_VISITED: Mutex<HashMap<String, PeerStatus>> = Mutex::new(HashMap::new());
}
pub static NB_ADDR_TO_TEST: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
#[derive(PartialEq)]
pub enum Status {
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

fn is_waiting(a_peer: String) -> bool {
    let mut address_visited = ADRESSES_VISITED.lock().unwrap();
    // println!("Before {:?}", address_visited);
    let mut is_waiting = false;
    if !address_visited.contains_key(&a_peer) {
        address_visited.insert(a_peer, PeerStatus{status:Status::Connecting, retries:0});
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
    address_status.insert(a_peer, PeerStatus{status:Status::Failed, retries:0});
}

pub fn done(a_peer :String) {
    let mut address_status = ADRESSES_VISITED.lock().unwrap();
    address_status.insert(a_peer, PeerStatus{status:Status::Done, retries:0});
}

pub fn get_peers_status() {
    let mut done = 0;
    let mut fail = 0;
    let mut other = 0;
    let address_status  = ADRESSES_VISITED.lock().unwrap();
    for (_, peer_status) in address_status.iter(){
        match peer_status.status {
            Status::Done => done += 1,
            Status::Failed => fail += 1,
            _ => other +=1,
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

pub fn register_peer_connection(a_peer: &String) {
    let mut address_status = ADRESSES_VISITED.lock().unwrap();
    address_status.insert(a_peer.to_string(), PeerStatus{status:Status::Connected, retries:0});
}

pub fn check_addr_messages(new_addresses: Vec<String>, address_channel: Sender<String>) -> usize {
    for new_peer in &new_addresses {
        if is_waiting(new_peer.to_string()) {

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
