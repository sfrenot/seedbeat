use std::sync::Mutex;
use lazy_static::lazy_static;
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian};
use std::time::SystemTime;
use sha2::{Sha256, Digest};
use std::net::TcpStream;
use std::io::{Write, Read, Error, ErrorKind};

use std::io::Cursor;
use hex::FromHex;

const VERSION:u32 = 70015;
const PORT:u16 = 8333;

// Timer
const MESSAGE_TIMEOUT:std::time::Duration = std::time::Duration::from_secs(120);

// services
const NODE_NETWORK:u64 = 1;
const NODE_BLOOM:u64 = 4;
const NODE_WITNESS:u64 = 8;
const NODE_NETWORK_LIMITED:u64  = 1024;

// payload struct
lazy_static! {
    static ref TEMPLATE_MESSAGE_PAYLOAD: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(105));
    static ref TEMPLATE_GETBLOCK_PAYLOAD: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(101));
}

const START_DATE:usize = 12;
const END_DATE:usize= 20;

// HEADER STRUCT
const HEADER_SIZE:usize = 24;
const MAGIC:&[u8;4]  =  &[0xF9, 0xBE, 0xB4, 0xD9];

const START_MAGIC:usize = 0;
const END_MAGIC:usize = 4;
const START_CMD:usize = 4;
const END_CMD:usize = 16;
const START_PAYLOAD_LENGTH :usize= 16;
const END_PAYLOAD_LENGTH :usize= 20;
const START_CHECKSUM:usize = 20;
const END_CHECKSUM:usize = 24;

// COMMANDS
pub const MSG_VERSION:&str = "version";
pub const MSG_VERSION_ACK:&str = "verack";
pub const MSG_GETADDR:&str = "getaddr";
pub const MSG_ADDR:&str = "addr";
pub const INV:&str = "inv";
pub const CONN_CLOSE:&str = "CONNCLOSED";
pub const GET_BLOCKS:&str = "getblocks";


// TO BE SUPPRESSED
const BLOCKS_ID:[&str;24] = [
"00000000000000000003430b1f8420b08d1e6f2bd28b739f4cec1e8de14507c8",
"00000000000000000002d3cec43724718cb473883c0b4b444a713e53cd27e085", //709982
"00000000000000000003ecbd35408b3f3bf44be841d464fa42174958f6cc99e1", //709981
"000000000000000000076ed16078f50ded3a5ef31902e229e01a35bdd43093c1", //709980
"00000000000000000005617eb7255be4dcd9096694c559e462e7c0456ab968a6", //709956
"00000000000000000002d28921f079781a992e44ac753e4d4e7b91b2be6e78e3", //709955
"000000000000000000084c07de5f063c4e1312584640499433a65f636eb86e2d", //709868
"000000000000000000020ddbbfb413f1deda472fcf7cb801f3088b8f322bf866", //709867
"0000000000000000000c36d591ad8c0c7330e84458ffde6c6c024d7ae888cffe", //709845
"000000000000000000055988c1b0ab4440f0f7583056c580e9a0aa6ac8683b57", //709388
"0000000000000000000387F16D9853CA4CCA63B7BC0AA7FBBB2268DF7FB0B3FD", //709387
"00000000000000000003A0B28AC8C3BE728FD8446CC68C6C3E1CD53A2A79E034", //709386
"0000000000000000000a482bf62cd477fd422ef92fa2a7e5e68038ecbbff5775", //709083
"0000000000000000000883AED93761D3DEECE45AE975D23FE408DA8D2A4EBEDD", //709082
"0000000000000000000381EB49D93C588D60A377A9BE00A3FAE6827856B7735E", //709081
"0000000000000000000017E3E40294241F45D1D9FB201A57E296588BF3A129C9", //709077
"0000000000000000000A66661135CC362AAB9D3A64837BC4A13C76957A0A4CD9", //709076
"0000000000000000000950D380817357CC6E9425B13098904B8192EAC4849198", //709043
"0000000000000000000f5922af9ae7762d251db95e17c9586fbbc2e89c66a9b7", //692917
"000000000000000000DA4BFF4FE47B622A37E51B13B44768C3B013D006FC0173", //460602
"000000006A625F06636B8BB6AC7B960A8D03705D1ACE08B1A19DA3FDCC99DDBD", //2
"00000000839A8E6886AB5951D76F411475428AFC90947EE320161BBF18EB6048", //1
"000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f", //0
"0000000000000000000000000000000000000000000000000000000000000000"
];

pub struct ReadResult {
    pub command: String,
    pub payload: Vec<u8>,
    pub error: Option<std::io::Error>
}

pub fn create_block_message_payload() {
    let mut block_message = TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap();

    block_message.extend(VERSION.swap_bytes().to_be_bytes());
    block_message.extend([BLOCKS_ID.len() as u8]);
    for elem in BLOCKS_ID {
        block_message.extend(Vec::from_hex(elem).unwrap());
    }
    // drop(block_message);
    // eprintln!("{:02x?}", hex::encode(TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().to_vec()));
    // std::process::exit(1);
}

pub fn create_init_message_payload() {

    let services:u64 = NODE_NETWORK | NODE_BLOOM | NODE_WITNESS | NODE_NETWORK_LIMITED;
    let date_buffer:u64 = 0;
    let address_buffer:u64 = 0;

    let binary_ip  = [127, 0, 0, 1];

    let mut address_from = Vec::from_hex("00000000000000000000ffff").unwrap();
    address_from.extend(binary_ip);
    address_from.extend(PORT.swap_bytes().to_be_bytes());

    let node_id = Vec::from_hex("1414141414141412").unwrap();
    let user_agent:&[u8] = "\x0C/bcpc:0.0.1/".as_bytes();
    let height:u32 = 708998;

    let mut message_payload = TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap();
    message_payload.extend(VERSION.swap_bytes().to_be_bytes());
    message_payload.extend(services.swap_bytes().to_be_bytes());
    message_payload.extend(date_buffer.swap_bytes().to_be_bytes());
    message_payload.extend(address_buffer.swap_bytes().to_be_bytes());
    message_payload.extend(services.swap_bytes().to_be_bytes());
    message_payload.extend(&address_from);
    message_payload.extend(services.swap_bytes().to_be_bytes());
    message_payload.extend(&address_from);
    message_payload.extend(node_id);
    message_payload.extend(user_agent);
    message_payload.extend(height.swap_bytes().to_be_bytes());

}

// Read message from a peer return command, payload, err
pub fn read_message(mut connection: &TcpStream) -> ReadResult {

    let mut read_result = ReadResult {
        command: String::new(),
        payload: Vec::new(),
        error: None
    };

    let mut header_buffer = [0 as u8;HEADER_SIZE];
    connection.set_read_timeout(Some(MESSAGE_TIMEOUT)).unwrap();
    return match connection.read(&mut header_buffer) {
        Ok(_) => {
            // println!("Lecture faite {:02X?}", header_buffer);
            if header_buffer[START_MAGIC..END_MAGIC] != MAGIC[..] {
                //println!("Error in Magic message header: {:?}", &header_buffer[START_MAGIC..END_MAGIC]);
                read_result.error = Some(Error::new(ErrorKind::Other, "Magic error"));
                return read_result
            }

            let cmd = String::from_utf8_lossy(&header_buffer[START_CMD..END_CMD]);
            let command = cmd.trim_matches(char::from(0));
            read_result.command = String::from(command);
            let mut payload_size_reader = Cursor::new(&header_buffer[START_PAYLOAD_LENGTH..END_PAYLOAD_LENGTH]);
            let payload_size = payload_size_reader.read_u32::<LittleEndian>().unwrap();

            if payload_size <= 0 { return read_result };

            let mut payload_buffer = vec![0u8; payload_size as usize];
            match connection.read_exact(&mut payload_buffer) {
                Ok(_) => {
                    read_result.payload = payload_buffer;
                    read_result
                }
                Err(e) => {
                    eprintln!("error reading payload");
                    read_result.error = Some(e);
                    read_result
                }
            }
        },
        Err(e) => {
            // eprintln!("error reading header {}", e);
            read_result.error = Some(e);
            return read_result
        }
    }
}

// Send request to a peer, return result with error or with bytes sent
pub fn send_request(mut connection: & TcpStream, message_name: &str) -> std::io::Result<usize> {
    let request:Vec<u8> = build_request(message_name);
    // if message_name == "getblocks" {
    //     eprintln!("-> {:02X?}", request);
    //     std::process::exit(0);
    // }
    let result = connection.write(request.as_slice());
    return result;
}


fn build_request(message_name : &str) -> Vec<u8>{
    let mut payload_bytes: Vec<u8> = Vec::new();
    if message_name == MSG_VERSION {
        payload_bytes = get_payload_with_current_date();
        // eprintln!("->MSG_VERSION : {:02X?}", payload_bytes);
    } else if message_name == GET_BLOCKS {
        payload_bytes = TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().clone();
        // eprintln!("==> GET_BLOCKS : {:02X?}", payload_bytes);
        // std::process::exit(1);
    }
    let mut header :Vec<u8> = vec![0; HEADER_SIZE];
    build_request_message_header(& mut header, message_name, &payload_bytes);
    let mut request = vec![];
    request.extend(header);
    request.extend(payload_bytes);
    // if message_name == GET_BLOCKS {
    //     // eprintln!("==> BEFORE SEND GET_BLOCKS: {:02X?}", request);
    //     // std::process::exit(1);
    // }
    return request;
}

fn get_payload_with_current_date() -> Vec<u8> {
    let mut payload :Vec<u8>  = TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().clone();
    let mut date :Vec<u8> = Vec::new();
    let unix_timestamp:u64 = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    date.write_u64::<LittleEndian>(unix_timestamp).unwrap();
    payload.splice(START_DATE..END_DATE, date.iter().cloned());
    return payload;
}

fn build_request_message_header(header: & mut Vec<u8>, command_name :&str, payload : &Vec<u8>){

    header.splice(START_MAGIC..END_MAGIC, MAGIC.iter().cloned());
    let end_cmd = command_name.as_bytes().len() +START_CMD;
    if end_cmd > END_CMD { panic!("wrong command") }
    header.splice(START_CMD..end_cmd, command_name.as_bytes().iter().cloned());

    let payload_len :u32 = payload.len() as u32;
    let mut payload_len_buffer = Vec::new();
    payload_len_buffer.write_u32::<LittleEndian>(payload_len).unwrap();
    let slice:&[u8] =&payload_len_buffer[..];
    header.splice(START_PAYLOAD_LENGTH..END_PAYLOAD_LENGTH, slice.iter().cloned());

    let checksum = compute_checksum(payload);
    header.splice(START_CHECKSUM..END_CHECKSUM, checksum.iter().cloned());

}

fn compute_checksum(payload : &Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(payload);
    let sum = hasher.result();
    let mut hasher2 = Sha256::new();
    hasher2.input(sum);
    let result = hasher2.result();
    return result[0..4].to_vec();
}
