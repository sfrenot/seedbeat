use std::sync::Mutex;
use std::sync::MutexGuard;

use lazy_static::lazy_static;
use hex::FromHex;
use crate::bcmessage::{VERSION, VERSION_END};
use std::collections::HashMap;

#[derive(Debug)]
pub struct BlockDesc {
    pub idx: usize,
    pub previous: String
}

lazy_static! {
    // static ref TEMPLATE_MESSAGE_PAYLOAD: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(105));
    static ref TEMPLATE_GETBLOCK_PAYLOAD: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(197));
    pub static ref BLOCKS_ID: Mutex<Vec<(String, bool)>> = {
        let mut m = Vec::with_capacity(5);
        m.push((String::from("0000000000000000000000000000000000000000000000000000000000000000"), false));
        Mutex::new(m)
    };
    pub static ref KNOWN_BLOCK: Mutex<HashMap<String, BlockDesc>> = Mutex::new(HashMap::new());
}

pub fn get_getblock_message_payload() -> Vec<u8> {
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().clone()
}

pub fn get_getheaders_message_payload() -> Vec<u8> {
    get_getblock_message_payload()
}

pub fn get_getdata_message_payload(search_block: &str) -> Vec<u8> {
    let mut block_message = Vec::with_capacity(37);
    block_message.extend([0x01]); //Number of Inventory vectors
    block_message.extend([0x02, 0x00, 0x00, 0x00]);
    let mut block = Vec::from_hex(search_block).unwrap();
    block.reverse();
    block_message.extend(block);
    block_message
}

pub fn create_block_message_payload(blocks_id: &Vec<(String, bool)>) {
    let mut block_message = TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap();
    *block_message = Vec::with_capacity(block_message.len()+32);
    block_message.extend(VERSION.to_le_bytes());
    block_message.extend([blocks_id.len() as u8-1]); // Fake value replaced later
    let size = blocks_id.len()-1;
    let mut nb = 0;
    for i in 0..blocks_id.len() {
        let (bloc, next) = &blocks_id[size-i];
        if !next {
            let mut val = Vec::from_hex(bloc).unwrap();
            val.reverse();
            block_message.extend(val);
            nb+=1;
        }
    }
    block_message[VERSION_END] = nb-1; //Vector size
    // drop(block_message);
    // eprintln!("{}",hex::encode(&get_getheaders_message_payload()));
    // std::process::exit(1);
}

pub fn is_new(known_block: &mut MutexGuard<HashMap<String, BlockDesc>>,blocks_id: &mut MutexGuard<Vec<(String, bool)>>, block: String, previous: String ) -> Result<usize, ()> {

    let search_block =  known_block.get(&block);
    let search_previous = known_block.get(&previous);

    match search_previous {
        Some(previous_block) => {
            match search_block {
                None => {
                    let (val, _) = blocks_id.get(previous_block.idx).unwrap();
                    blocks_id[previous_block.idx] =  (val.to_string(), true);                    // std::mem::replace(&mut blocks_id[previous_block.idx], (val.to_string(), true));
                    blocks_id.insert((previous_block.idx+1) as usize, (block.clone(), false));

                    let idx = previous_block.idx + 1;
                    known_block.insert(block.clone(), BlockDesc{idx, previous});
                    // eprintln!("Trouvé previous, Pas trouvé block");
                    // eprintln!("{:?}", blocks_id);
                    // eprintln!("{:?}", known_block);
                    // std::process::exit(1);
                    Ok(idx)
                }
                _ => {
                    Ok(0)
                }
            }
        }
        _ => {
            match search_block {
                Some(found_block) => {
                    // eprintln!("Previous {} non trouvé, Block trouvé {}", &previous, &block);
                    let idx = found_block.idx;
                    let val = BlockDesc{idx, previous: previous.clone()};
                    known_block.insert(block.clone(), val);

                    blocks_id.insert(idx, (previous.clone(), true));
                    eprintln!("Previous non {}, Block oui {}", &previous, &block);

                    // eprintln!("{:?}", blocks_id);
                    // eprintln!("{:?}", known_block);
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Previous non {}, Block non {}", &previous, &block);
                    // eprintln!("{:?}", blocks_id);
                    // eprintln!("{:?}", known_block);
                    Err(())
                }
            }
        }
    }
}
