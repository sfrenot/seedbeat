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
use bcmessage::{ReadResult, INV, MSG_VERSION, MSG_VERSION_ACK, MSG_GETADDR, CONN_CLOSE, MSG_ADDR, GET_HEADERS, HEADERS, GET_BLOCKS, BLOCK, GET_DATA};
use crate::bcfile as bcfile;
use crate::bcblocks as bcblocks;
use crate::bcpeers as bcpeers;

const CONNECTION_TIMEOUT:Duration = Duration::from_secs(10);
const MESSAGE_TIMEOUT:Duration = Duration::from_secs(120);
const MIN_ADDRESSES_RECEIVED_THRESHOLD: usize = 5;
const NB_MAX_READ_ON_SOCKET:usize = 300;


pub fn handle_one_peer(connection_start_channel: Receiver<String>, address_channel_tx: Sender<String>, _num: u64){

    loop{ //Nodes Management
        let target_address = connection_start_channel.recv().unwrap();
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

            let mut connection = result.unwrap();
            let (in_chain_sender, in_chain_receiver) = mpsc::channel();
            let target = target_address.clone();
            let connection_clone = connection.try_clone().unwrap();
            let sender = address_channel_tx.clone();
            thread::spawn(move || {
                handle_incoming_message(&connection_clone, target, in_chain_sender, sender);
            });

            loop { //Connection management
                match connection_hello(&connection,&in_chain_receiver) {
                    Err(_e) =>  {
                        // eprintln!("Error sending request: {}: {}", _e, target_address);
                        bcpeers::fail(target_address.clone());
                        break;
                    },
                    _ => {}
                }

                match connection.write(bcmessage::build_request(MSG_GETADDR).as_slice()) {
                    Err(_) => {
                        eprintln!("error at sending getaddr: {}", target_address);
                        bcpeers::fail(target_address.clone());
                        break; // From connexion
                    }
                    _ => {}
                }

                loop { // Handle block Exchanges
                    match in_chain_receiver.recv().unwrap() {
                        cmd if cmd == String::from(GET_HEADERS) => {
                            // eprintln!("==> Envoi GET_HEADERS {} to: {}", received_cmd, target_address);
                            match connection.write(bcmessage::build_request(GET_HEADERS).as_slice()) {
                                Err(_) => {
                                    println!("error at sending getHeaders");
                                    bcpeers::fail(target_address.clone());
                                    break; // From connexion
                                }
                                _ => {}
                            };
                        },
                        cmd if cmd == String::from(GET_BLOCKS) => {
                            // eprintln!("==> Envoi GET_BLOCKS {} to: {}", received_cmd, target_address);
                            match connection.write(bcmessage::build_request(GET_BLOCKS).as_slice()) {
                                Err(_) => {
                                    println!("error at sending getaddr");
                                    bcpeers::fail(target_address.clone());
                                    break; // From connexion
                                }
                                _ => {}
                            };
                        },
                        cmd if cmd == String::from(GET_BLOCKS) => {
                            // eprintln!("==> Envoi GET_BLOCKS {} to: {}", received_cmd, target_address);
                            match connection.write(bcmessage::build_request(GET_BLOCKS).as_slice()) {
                                Err(_) => {
                                    println!("error at sending getaddr");
                                    bcpeers::fail(target_address.clone());
                                    break; // From connexion
                                }
                                _ => {}
                            };
                        },
                        cmd if &cmd[..GET_DATA.len()] == String::from(GET_DATA) => {
                            eprintln!("Recherche info block {}", &cmd[GET_DATA.len()+1..]);
                            match connection.write(bcmessage::build_request(&cmd).as_slice()) {
                                Err(_) => {
                                    println!("error at sending getData");
                                    bcpeers::fail(target_address.clone());
                                    break; // From connexion
                                }
                                _ => {}
                            };
                        },
                        cmd if cmd == String::from(CONN_CLOSE) => {
                            // eprintln!("Fermeture {}", &target_address);
                            bcpeers::done(target_address.clone());
                            break; // From connexion
                        },
                        cmd => {
                            println!("Bad message {}", cmd);
                            std::process::exit(1);
                        }
                    };
                } //loop for internal mesages
                break;
            } // loop for node
        }
        // eprintln!("Fin gestion {}", target_address);
        bcpeers::NB_ADDR_TO_TEST.fetch_sub(1, Ordering::Relaxed);
    }
}

fn handle_incoming_message(connection:& TcpStream, target_address: String, in_chain: Sender<String>, sender: Sender<String>)  {
    connection.set_read_timeout(Some(MESSAGE_TIMEOUT)).unwrap();
    let mut lecture = 0; // Garde pour éviter connection infinie inutile
    loop {
        // eprintln!("Attente lecture sur {}", target_address);
        let read_result:ReadResult = bcmessage::read_message(&connection);
        // println!("Lecture de {}", target_address);

        match read_result.error {
            Some(_error) => {
                // eprintln!("Erreur Lecture {}: {}", _error, target_address);
                // in_chain.send(String::from(CONN_CLOSE)).unwrap();
                match in_chain.send(String::from(CONN_CLOSE)) {
                    Err(err) => {
                        eprintln!("Erreur incoming, end : {}", err);
                    }
                    _ => {}
                }
                break;
            }
            _ => {
                let command = read_result.command;
                let payload = read_result.payload;
                lecture+=1;
                // eprintln!("Command From : {} --> {}, payload : {}", &target_address, &command, payload.len());
                if command  == String::from(MSG_VERSION) && payload.len() > 0 {
                    let peer = target_address.clone();
                    bcfile::store_version_message(peer, bcmessage::process_version_message(&payload));
                    bcpeers::register_peer_connection(target_address.clone());
                    // eprintln!("Envoi MSG_VERSION {}", target_address);
                    in_chain.send(command).unwrap();
                    continue;
                }
                if command == String::from(MSG_VERSION_ACK) {
                    // eprintln!("Envoi MSG_VERSION_ACK {}", target_address);
                    in_chain.send(command).unwrap();
                    continue;
                }
                if command == String::from(MSG_ADDR)  && payload.len() > 0 {
                    if bcpeers::check_addr_messages(bcmessage::process_addr_message(&payload), sender.clone()) > MIN_ADDRESSES_RECEIVED_THRESHOLD {
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

                if command == String::from(HEADERS)  && payload.len() > 0 {
                    let mut known_block_guard = bcblocks::KNOWN_BLOCK.lock().unwrap();
                    let mut blocks_id_guard = bcblocks::BLOCKS_ID.lock().unwrap();

                    // eprintln!("Status : {} -> {}", idx, block);
                    match bcmessage::process_headers_message(&mut known_block_guard, &mut blocks_id_guard, payload) {
                        Ok(()) => {
                            match bcfile::store_blocks(&blocks_id_guard) {
                               true => {
                                   bcblocks::create_block_message_payload(&blocks_id_guard);
                                   eprintln!("new payload -> {:02x?}", hex::encode(&bcblocks::get_getheaders_message_payload()));
                                   lecture = 0;
                               },
                               false => {
                                   std::process::exit(1);
                               }
                           };
                        },
                        Err(err) => {
                            match err {
                                bcmessage::ProcessHeadersMessageError::UnkownBlocks => {
                                    eprintln!("Sortie du noeud");
                                    in_chain.send(String::from(CONN_CLOSE)).unwrap();
                                    break;
                                },
                                _ => {}
                            };
                        }
                    };
                    in_chain.send(String::from(GET_HEADERS)).unwrap();
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
            in_chain.send(String::from(CONN_CLOSE)).unwrap();
            break;
        }
    }
    // eprintln!("Fermeture {}", target_address);
}

fn connection_hello(connection:&TcpStream, in_chain_receiver: &mpsc::Receiver<String>) -> Result<(), Error>{
    pingpong(&connection, &in_chain_receiver, MSG_VERSION)?;
    pingpong(&connection, &in_chain_receiver, MSG_VERSION_ACK)
}

fn pingpong(mut connection:&TcpStream, in_chain_receiver: &mpsc::Receiver<String>, msg: &str) -> Result<(), Error>{
    connection.write(bcmessage::build_request(msg).as_slice())?; //ping
    match in_chain_receiver.recv().unwrap() {  //pong
        res if msg == res => return Ok(()),
        res => return Err(Error::new(ErrorKind::Other, format!("Wrong message {} <> {}", msg, res)))
    };
}
