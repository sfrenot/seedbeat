use std::sync::Mutex;
use lazy_static::lazy_static;
use hex::FromHex;
use crate::bcmessage::{VERSION};
use std::collections::HashMap;

lazy_static! {
    // static ref TEMPLATE_MESSAGE_PAYLOAD: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(105));
    pub static ref TEMPLATE_GETBLOCK_PAYLOAD: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(197));
    static ref BLOCKS_ID: Mutex<Vec<String>> = {
        let mut m = Vec::with_capacity(5);
        m.push(String::from("0000000000000000000000000000000000000000000000000000000000000000"));
        m.push(String::from("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f")); //0
        m.push(String::from("00000000839A8E6886AB5951D76F411475428AFC90947EE320161BBF18EB6048")); //1
        m.push(String::from("000000006A625F06636B8BB6AC7B960A8D03705D1ACE08B1A19DA3FDCC99DDBD")); //2
        m.push(String::from("00000000000000000bf9a116fc80506cc10962ed753a0d5dd0ad71339b59e5df")); //360902
        m.push(String::from("00000000000000000009177dcfc80ebd31d20a7abcfd515019d9737fa09a68d3")); //710015
        m.push(String::from("000000000000000000086e525463f00da70f517593cdf4f2b1416af398423a8a")); //710139
        Mutex::new(m)
    };
    static ref KNOWN_BLOCK: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

// TO BE SUPPRESSED
// const BLOCKS_ID:[&str;4] = [
// // "00000000000000000009177dcfc80ebd31d20a7abcfd515019d9737fa09a68d3", //710015
// // "0000000000000000000945eb41375ad89bcc042ed1bf9baf08ab9dbd58b61b5a", //710006
// // "000000000000000000081da14bccec2c1b87a082065a74c48718ff5440dfb8ee", //710004
// // "00000000000000000000cfeab623bcb7ff4cba31e5a7f87788a4f714435aae77", //710003
// // "00000000000000000003430b1f8420b08d1e6f2bd28b739f4cec1e8de14507c8", //709983
// // "00000000000000000002d3cec43724718cb473883c0b4b444a713e53cd27e085", //709982
// // "00000000000000000003ecbd35408b3f3bf44be841d464fa42174958f6cc99e1", //709981
// // "000000000000000000076ed16078f50ded3a5ef31902e229e01a35bdd43093c1", //709980
// // "00000000000000000005617eb7255be4dcd9096694c559e462e7c0456ab968a6", //709956
// // "00000000000000000002d28921f079781a992e44ac753e4d4e7b91b2be6e78e3", //709955
// // "000000000000000000084c07de5f063c4e1312584640499433a65f636eb86e2d", //709868
// // "000000000000000000020ddbbfb413f1deda472fcf7cb801f3088b8f322bf866", //709867
// // "0000000000000000000c36d591ad8c0c7330e84458ffde6c6c024d7ae888cffe", //709845
// // "000000000000000000055988c1b0ab4440f0f7583056c580e9a0aa6ac8683b57", //709388
// // "0000000000000000000387F16D9853CA4CCA63B7BC0AA7FBBB2268DF7FB0B3FD", //709387
// // "00000000000000000003A0B28AC8C3BE728FD8446CC68C6C3E1CD53A2A79E034", //709386
// // "0000000000000000000a482bf62cd477fd422ef92fa2a7e5e68038ecbbff5775", //709083
// // "0000000000000000000883AED93761D3DEECE45AE975D23FE408DA8D2A4EBEDD", //709082
// // "0000000000000000000381EB49D93C588D60A377A9BE00A3FAE6827856B7735E", //709081
// // "0000000000000000000017E3E40294241F45D1D9FB201A57E296588BF3A129C9", //709077
// // "0000000000000000000A66661135CC362AAB9D3A64837BC4A13C76957A0A4CD9", //709076
// // "0000000000000000000950D380817357CC6E9425B13098904B8192EAC4849198", //709043
// // "0000000000000000000f5922af9ae7762d251db95e17c9586fbbc2e89c66a9b7", //692917
// // "000000000000000000DA4BFF4FE47B622A37E51B13B44768C3B013D006FC0173", //460602
//
// "000000006A625F06636B8BB6AC7B960A8D03705D1ACE08B1A19DA3FDCC99DDBD", //2
// "00000000839A8E6886AB5951D76F411475428AFC90947EE320161BBF18EB6048", //1
// "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f", //0
// "0000000000000000000000000000000000000000000000000000000000000000"
// ];
pub fn get_getblock_message_payload() -> Vec<u8> {
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().clone()
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

pub fn create_block_message_payload(new_block_t: Option<String>) {
    let mut blocks_id = BLOCKS_ID.lock().unwrap();

    match new_block_t {
        Some(new_block) => blocks_id.push(new_block),
        None => {}
    }
    let mut block_message = TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap();

    *block_message = Vec::with_capacity(block_message.len()+32);

    block_message.extend(VERSION.to_le_bytes());
    block_message.extend([blocks_id.len() as u8]);
    let size = blocks_id.len()-1;
    for i in 0..blocks_id.len() {
        let val = &blocks_id[size-i];
        block_message.extend(Vec::from_hex(val).unwrap());
    }

    // match new_block_t {
    //     Some(_) => {
    //         drop(block_message);
    //         eprintln!("{:02x?}", hex::encode(TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().to_vec()));
    //         std::process::exit(1);
    //     }
    //     None => {}
    // }
}

pub fn is_new(block_name: String) -> bool {
    // true: needs search
    let res: bool;
    let mut known_block = KNOWN_BLOCK.lock().unwrap();
    // eprintln!("Test Block ==> {}", &block_name);
    match known_block.get(&block_name) {
        None => {
            eprintln!("Ajout");
            known_block.insert(block_name.clone(), true);
            create_block_message_payload(Some(block_name));
            res = true;
        }
        Some(found) => {
            res = *found;
        }
    }
    eprintln!("Hash {:?}", &known_block);
    res
}
