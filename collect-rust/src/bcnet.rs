pub mod bcmessage;

use std::time::Duration;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::atomic::Ordering;
use std::thread;
use std::io::Write;
use std::io::Error;
use std::io::ErrorKind;


use std::net::{SocketAddr, TcpStream};
use chan::Receiver;

// use crate::bcmessage as bcmessage;
// use crate::bcmessage::{ReadResult, INV, MSG_VERSION, MSG_VERSION_ACK, MSG_GETADDR, CONN_CLOSE, MSG_ADDR, HEADERS, GET_BLOCKS, BLOCK, GET_DATA};
use bcmessage::{INV, MSG_VERSION, MSG_VERSION_ACK, MSG_GETADDR, CONN_CLOSE, MSG_ADDR, GET_HEADERS, HEADERS, GET_BLOCKS, BLOCK, GET_DATA};
use crate::bcfile as bcfile;
use crate::bcblocks as bcblocks;
use crate::bcpeers as bcpeers;

const CONNECTION_TIMEOUT:Duration = Duration::from_secs(10);
const MESSAGE_TIMEOUT:Duration = Duration::from_secs(120);
const MIN_ADDRESSES_RECEIVED_THRESHOLD: usize = 5;
const NB_MAX_READ_ON_SOCKET:usize = 300;


// pub fn handle_one_peer(connection_start_channel: Receiver<String>, address_channel_tx: Sender<String>, _num: u64){
//
//     loop{ //Nodes Management
//         let target_address = connection_start_channel.recv().unwrap();
//         // eprintln!("Connexion {}, {}", _num, target_address);
//         let socket: SocketAddr = target_address.parse().unwrap();
//         let result = TcpStream::connect_timeout(&socket, CONNECTION_TIMEOUT);
//         // eprintln!("Connecté {}, {}", num, target_address);
//         if result.is_err() {
//             // println!(" {} -> Fail", target_address);
//             // println!(" -> Fail to connect {}: {}", target_address, result.err().unwrap());
//             bcpeers::fail(target_address.clone());
//         } else {
//             // println!(" {} -> Success", target_address);
//             // println!("Fail to connect {}: {}", target_address, result.err().unwrap());
//
//             let mut connection = result.unwrap();
//
//             let (in_chain_sender, in_chain_receiver) = mpsc::channel();
//             let target = target_address.clone();
//             let connection_clone = connection.try_clone().unwrap();
//             let sender = address_channel_tx.clone();
//             thread::spawn(move || {
//                 handle_incoming_message(&connection_clone, target, in_chain_sender, sender);
//             });
//
//             loop {
//                 //Connection management
//                 match connection_hello(&connection,&in_chain_receiver) {
//                     Err(_e) =>  {
//                         // eprintln!("Error sending request: {}: {}", _e, target_address);
//                         bcpeers::fail(target_address.clone());
//                         break;
//                     },
//                     _ => {}
//                 }
//
//                 match connection.write(bcmessage::build_request(&MSG_GETADDR).as_slice()) {
//                     Err(_) => {
//                         eprintln!("error at sending getaddr: {}", target_address);
//                         bcpeers::fail(target_address.clone());
//                         break; // From connexion
//                     }
//                     _ => {}
//                 }
//
//                 loop { // Handle block Exchanges
//
//                     let cmd = in_chain_receiver.recv().unwrap();
//                     match connection.write(bcmessage::build_request(&cmd).as_slice()) {
//                         Err(_) => {
//                             bcpeers::fail(target_address);
//                             break; // From connexion
//                         },
//                         _ => {
//                             if cmd == *CONN_CLOSE {
//                                 bcpeers::done(target_address.clone());
//                                 break;
//                             }
//                         }
//                     };
//
//                     //Special Case
//                     // cmd if &cmd[..GET_DATA.len()] == *GET_DATA => {
//                     //     eprintln!("Recherche info block {}", &cmd[GET_DATA.len()+1..]);
//                     //     match connection.write(bcmessage::build_request(&cmd).as_slice()) {
//                     //         Err(_) => {
//                     //             println!("error at sending getData");
//                     //             bcpeers::fail(target_address.clone());
//                     //             break; // From connexion
//                     //         }
//                     //         _ => {}
//                     //     };
//                     // }
//
//                 } //loop for internal mesages
//                 break;
//             } // loop for node
//         }
//         // eprintln!("Fin gestion {}", target_address);
//         bcpeers::NB_ADDR_TO_TEST.fetch_sub(1, Ordering::Relaxed);
//     }
// }

pub fn handle_one_peer(connection_start_channel: Receiver<String>, address_channel_tx: Sender<String>, _num: u64){

    loop{ //Nodes Management
        let target_address = connection_start_channel.recv().unwrap();
        let mut status: &String = &MSG_VERSION; // Start from this status

        // eprintln!("Connexion {}, {}", _num, target_address);
        let socket: SocketAddr = target_address.parse().unwrap();
        let result = TcpStream::connect_timeout(&socket, CONNECTION_TIMEOUT);
        // eprintln!("Connecté {}, {}", num, target_address);
        if result.is_err() {
            // println!(" {} -> Fail", target_address);
            // println!(" -> Fail to connect {}: {}", target_address, result.err().unwrap());
            bcpeers::fail(target_address.clone());
        } else {
            // println!(" {} -> Success", target_address);
            // println!("Fail to connect {}: {}", target_address, result.err().unwrap());

            let connection = result.unwrap();
            let (in_chain_sender, in_chain_receiver) = mpsc::channel();
            let target = target_address.clone();
            let connection_clone = connection.try_clone().unwrap();
            let sender = address_channel_tx.clone();
            thread::spawn(move || {
                handle_incoming_message(&connection_clone, target, in_chain_sender, sender);
            });

            loop {
               eprintln!("Avant Activation {}, {}", target_address.clone(), status);
               match activate_peer(&connection, &in_chain_receiver, &status) {
                   Err(e) => {
                       match e.kind() {
                           ErrorKind::Other => {
                               eprintln!("Fin du noeud: {}: {}", e, target_address);
                               bcpeers::done(target_address.clone());
                               break;
                           },
                           _ => {
                               eprintln!("Error sending request: {}: {}", e, target_address);
                               bcpeers::fail(target_address.clone());
                               break;
                           }
                       }
                   },
                   Ok(new_status) => {
                       status = &new_status;
                       eprintln!("Après Activation {}, {}", target_address.clone(), status);
                   }
               }
                //
                //
                // //Connection management
                // match connection_hello(&connection,&in_chain_receiver) {
                //     Err(_e) =>  {
                //         // eprintln!("Error sending request: {}: {}", _e, target_address);
                //         bcpeers::fail(target_address.clone());
                //         break;
                //     },
                //     _ => {}
                // }
                //
                // match connection.write(bcmessage::build_request(&MSG_GETADDR).as_slice()) {
                //     Err(_) => {
                //         eprintln!("error at sending getaddr: {}", target_address);
                //         bcpeers::fail(target_address.clone());
                //         break; // From connexion
                //     }
                //     _ => {}
                // }
                //
                // loop { // Handle block Exchanges
                //
                //     let cmd = in_chain_receiver.recv().unwrap();
                //     match connection.write(bcmessage::build_request(&cmd).as_slice()) {
                //         Err(_) => {
                //             bcpeers::fail(target_address);
                //             break; // From connexion
                //         },
                //         _ => {
                //             if cmd == *CONN_CLOSE {
                //                 bcpeers::done(target_address.clone());
                //                 break;
                //             }
                //         }
                //     };
                //
                //     //Special Case
                //     // cmd if &cmd[..GET_DATA.len()] == *GET_DATA => {
                //     //     eprintln!("Recherche info block {}", &cmd[GET_DATA.len()+1..]);
                //     //     match connection.write(bcmessage::build_request(&cmd).as_slice()) {
                //     //         Err(_) => {
                //     //             println!("error at sending getData");
                //     //             bcpeers::fail(target_address.clone());
                //     //             break; // From connexion
                //     //         }
                //     //         _ => {}
                //     //     };
                //     // }
                //
                // } //loop for internal mesages
                // break;
            } // loop for node
        }
        // eprintln!("Fin gestion {}", target_address);
        bcpeers::NB_ADDR_TO_TEST.fetch_sub(1, Ordering::Relaxed);
    }
}

fn handle_incoming_message(connection:& TcpStream, target_address: String, in_chain: Sender<String>, sender: Sender<String>)  {
    connection.set_read_timeout(Some(MESSAGE_TIMEOUT)).unwrap();
    let mut lecture:usize = 0; // Garde pour éviter connection infinie inutile
    loop {
        // eprintln!("Attente lecture sur {}", target_address);
        // let read_result:ReadResult = bcmessage::read_message(&connection);
        // println!("Lecture de {}", target_address);

        match bcmessage::read_message(&connection) {
            Err(_error) => {
                // eprintln!("Erreur Lecture {}: {}", _error, target_address);
                in_chain.send((*CONN_CLOSE).clone()).unwrap();
                break;
            },
            Ok((command, payload)) => {
                lecture+=1;
                // eprintln!("Command From : {} --> {}, payload : {}", &target_address, &command, payload.len());
                if command == *MSG_VERSION && payload.len() > 0 {
                    handle_incoming_cmd_version(&target_address, &payload);
                    in_chain.send(command).unwrap();
                    continue;
                }
                if command == *MSG_VERSION_ACK {
                    // eprintln!("Envoi MSG_VERSION_ACK {}", target_address);
                    in_chain.send(command).unwrap();
                    continue;
                }

                if command == *MSG_ADDR && payload.len() > 0 && handle_incoming_cmd_msg_addr(&payload, &sender){
                    in_chain.send((*MSG_GETADDR).clone()).unwrap();
                    continue;
                }

                //Testing incoming message
                // if command == String::from(GET_HEADERS){
                //     eprintln!("GET-HEADERS {}", hex::encode(&payload));
                //     std::process::exit(1);
                // }

                if command == *HEADERS  && payload.len() > 0  {
                    if handle_incoming_cmd_msg_header(&payload, &mut lecture) {
                        in_chain.send((*GET_HEADERS).clone()).unwrap();
                    } else {
                        in_chain.send((*CONN_CLOSE).clone()).unwrap();
                        break;
                    }
                    continue;
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
        // eprintln!("-> Nouvelle lecture {} -> {}", target_address, lecture);
        if lecture > NB_MAX_READ_ON_SOCKET {
            eprintln!("Sortie du noeud : trop de lectures inutiles");
            in_chain.send((*CONN_CLOSE).clone()).unwrap();
            break;
        }
    }
    // eprintln!("Fermeture {}", target_address);
}

fn next_status(from: &String) -> &String {
    match from {
        elem if *elem == *MSG_VERSION => {&MSG_VERSION_ACK},
        elem if *elem == *MSG_VERSION_ACK => {&MSG_GETADDR},
        elem if *elem == *MSG_GETADDR => {&GET_HEADERS},
        elem if *elem == *GET_HEADERS => {&GET_HEADERS},
        _ => {&CONN_CLOSE}
    }
}

fn activate_peer<'a>(mut connection: &TcpStream, in_chain_receiver: &mpsc::Receiver<String>, current: &'a String) -> Result<&'a String, Error> {
    connection.write(bcmessage::build_request(current).as_slice())?; //ping
    match in_chain_receiver.recv().unwrap() {  //pong
        res if res == *CONN_CLOSE => Err(Error::new(ErrorKind::Other, format!("Connexion terminée {} <> {}", current, res))),
        res if res == *current => Ok(next_status(current)),
        res => Err(Error::new(ErrorKind::ConnectionReset, format!("Wrong message {} <> {}", current, res)))
    }
}


// Hello initial command from main handler
fn connection_hello(connection:&TcpStream, in_chain_receiver: &mpsc::Receiver<String>) -> Result<(), Error>{
    pingpong(&connection, &in_chain_receiver, &MSG_VERSION)?;
    pingpong(&connection, &in_chain_receiver, &MSG_VERSION_ACK)
}

fn pingpong(mut connection:&TcpStream, in_chain_receiver: &mpsc::Receiver<String>, msg: &str) -> Result<(), Error>{
    connection.write(bcmessage::build_request(msg).as_slice())?; //ping
    match in_chain_receiver.recv().unwrap() {  //pong
        res if msg == res => return Ok(()),
        res => return Err(Error::new(ErrorKind::Other, format!("Wrong message {} <> {}", msg, res)))
    };
}

// Incoming messages

fn handle_incoming_cmd_version(peer: &String, payload: &Vec<u8>) {
    bcfile::store_version_message(peer, bcmessage::process_version_message(payload));
    bcpeers::register_peer_connection(peer);
}

fn handle_incoming_cmd_msg_addr(payload: &Vec<u8>, sender: &Sender<String>) -> bool {
    bcpeers::check_addr_messages(bcmessage::process_addr_message(&payload), &sender) > MIN_ADDRESSES_RECEIVED_THRESHOLD
}

fn handle_incoming_cmd_msg_header(payload: &Vec<u8>, lecture: &mut usize) -> bool {
    let mut known_block_guard = bcblocks::KNOWN_BLOCK.lock().unwrap();
    let mut blocks_id_guard = bcblocks::BLOCKS_ID.lock().unwrap();

    // eprintln!("Status : {} -> {}", idx, block);
    match bcmessage::process_headers_message(&mut known_block_guard, &mut blocks_id_guard, payload) {
        Ok(()) => {
            match bcfile::store_blocks(&blocks_id_guard) {
               true => {
                   bcblocks::create_block_message_payload(&blocks_id_guard);
                   eprintln!("new payload -> {:02x?}", hex::encode(&bcblocks::get_getheaders_message_payload()));
                   *lecture = 0;
                   true
               },
               false => {
                   std::process::exit(1);
               }
           }
        },
        Err(err) => {
            match err {
                bcmessage::ProcessHeadersMessageError::UnkownBlocks => {
                    eprintln!("Sortie du noeud");
                    false
                },
                _ => {
                    // eprintln!("Erreur -> {:?}", err);
                    // std::process::exit(1);
                    true
                }
            }
        }
    }
}
