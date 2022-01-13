pub mod bcmessage;

use std::time::Duration;
use std::sync::mpsc::Sender;
use std::sync::atomic::Ordering;
use std::io::Write;
use std::io::Error;
use std::io::ErrorKind;

use std::net::{SocketAddr, TcpStream};
use chan::Receiver;

// use crate::bcmessage::{ReadResult, INV, MSG_VERSION, MSG_VERSION_ACK, MSG_GETADDR, CONN_CLOSE, MSG_ADDR, HEADERS, GET_BLOCKS, BLOCK, GET_DATA};
use bcmessage::{INV, MSG_VERSION, MSG_VERSION_ACK, MSG_GETADDR, CONN_CLOSE, MSG_ADDR, GET_HEADERS, HEADERS, GET_BLOCKS, BLOCK, GET_DATA};
use crate::bcfile as bcfile;
use crate::bcblocks as bcblocks;
use crate::bcpeers as bcpeers;

const CONNECTION_TIMEOUT:Duration = Duration::from_secs(10);
const MESSAGE_TIMEOUT:Duration = Duration::from_secs(120);
const MIN_ADDRESSES_RECEIVED_THRESHOLD: usize = 5;
const NB_MAX_READ_ON_SOCKET:usize = 300;

pub fn handle_one_peer(connection_start_channel: Receiver<String>, address_channel_tx: Sender<String>, _num: u64){
    loop{ //Node Management
        let target_address = connection_start_channel.recv().unwrap();
        let mut status: &String = &MSG_VERSION; // Start from this status

        // eprintln!("Connexion {}, {}", _num, target_address);
        let socket: SocketAddr = target_address.parse().unwrap();
        match TcpStream::connect_timeout(&socket, CONNECTION_TIMEOUT) {
            Err(_) => bcpeers::fail(target_address.clone()),
            Ok(connection) => {
                loop {
                   // eprintln!("Avant Activation {}, {}", target_address.clone(), status);
                   status = match activate_peer(&connection, &status, &address_channel_tx, &target_address) {
                       Err(e) => {
                           match e.kind() {
                               ErrorKind::Other => {
                                   // eprintln!("Fin du noeud: {}: {}", e, target_address);
                                   bcpeers::done(target_address.clone());
                               },
                               _ => {
                                   // eprintln!("Error sending request: {}: {}", e, target_address);
                                   bcpeers::fail(target_address.clone());
                               }
                           }
                           break;
                       },
                       Ok(new_status) =>{ &new_status }
                   }
                } // loop for node
            }
        };
        // eprintln!("Connecté {}, {}", num, target_address);
        // eprintln!("Fin gestion {}", target_address);
        bcpeers::NB_ADDR_TO_TEST.fetch_sub(1, Ordering::Relaxed);
    }
}

fn handle_incoming_message<'a>(connection:& TcpStream, sender: &Sender<String>, target_address: &String) -> &'a String  {
    connection.set_read_timeout(Some(MESSAGE_TIMEOUT)).unwrap();
    let mut lecture:usize = 0; // Garde pour éviter connection infinie inutile
    loop {
        // println!("Lecture de {}", target_address);
        match bcmessage::read_message(&connection) {
            Err(_error) => return &CONN_CLOSE,
            Ok((command, payload)) => {
                lecture+=1;
                //eprintln!("Command From : {} --> {}, payload : {}", &target_address, &command, payload.len());
                match command {
                    cmd if cmd == *MSG_VERSION && payload.len() > 0 => {
                        handle_incoming_cmd_version(&target_address, &payload);
                        return &MSG_VERSION;
                    },
                    cmd if cmd == *MSG_VERSION_ACK
                        => return &MSG_VERSION_ACK,
                    cmd if cmd == *MSG_ADDR && payload.len() > 0 && handle_incoming_cmd_msg_addr(&payload, &sender)
                        => return &MSG_GETADDR,
                    cmd if cmd == *HEADERS  && payload.len() > 0
                        => return match handle_incoming_cmd_msg_header(&payload, &mut lecture) {
                            true  => &GET_HEADERS,
                            false => &CONN_CLOSE
                        },
                    _ => {}
                };
                //
                // //Testing incoming message
                // // if command == String::from(GET_HEADERS){
                // //     eprintln!("GET-HEADERS {}", hex::encode(&payload));
                // //     std::process::exit(1);
                // // }
                //

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
        };
        if lecture > NB_MAX_READ_ON_SOCKET {
            eprintln!("Sortie du noeud : trop de lectures inutiles");
            return &CONN_CLOSE;
        }
    }
    // eprintln!("Fermeture {}", target_address);
}

// TODO: -> has a hashmap
fn next_status(from: &String) -> &String {
    match from {
        elem if *elem == *MSG_VERSION => {&MSG_VERSION_ACK},
        elem if *elem == *MSG_VERSION_ACK => {&MSG_GETADDR},
        elem if *elem == *MSG_GETADDR => {&GET_HEADERS},
        elem if *elem == *GET_HEADERS => {&GET_HEADERS},
        _ => {&CONN_CLOSE}
    }
}

fn activate_peer<'a>(mut connection: &TcpStream, current: &'a String, sender: &Sender<String>, target: &String) -> Result<&'a String, Error> {
    connection.write(bcmessage::build_request(current).as_slice())?;

    match handle_incoming_message(connection, sender, target) {
        res if *res == *CONN_CLOSE => Err(Error::new(ErrorKind::Other, format!("Connexion terminée {} <> {}", current, res))),
        res if *res == *current => Ok(next_status(current)),
        res if *res == *MSG_GETADDR && *current == *GET_HEADERS => Ok(current), // Remote node answers many times the same thing
        res => Err(Error::new(ErrorKind::ConnectionReset, format!("Wrong message {} <> {}", current, res)))
    }
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
