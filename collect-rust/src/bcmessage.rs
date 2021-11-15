use std::sync::Mutex;
use lazy_static::lazy_static;
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian};
use std::time::SystemTime;
use sha2::{Sha256, Digest};
use std::net::TcpStream;
use std::io::{Write, Read, Error, ErrorKind};

use std::io::Cursor;
use hex::FromHex;

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

pub struct ReadResult {
    pub command: String,
    pub payload: Vec<u8>,
    pub error: Option<std::io::Error>
}

pub fn init() {
    let version:u32 = 70015;
    let services:u64 = NODE_NETWORK | NODE_BLOOM | NODE_WITNESS | NODE_NETWORK_LIMITED;
    let date_buffer :u64 = 0;
    let address_buffer:u64 = 0;
    let address_prefix:Vec<u8>  =  vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF];
    let binary_ip:Vec<u8>  = vec![127, 0, 0, 1];
    let mut binary_port:Vec<u8>  =  vec![];
    binary_port.write_u16::<LittleEndian>(8333).unwrap();
    let mut address_from:Vec<u8>  = address_prefix.clone();
    address_from.extend(binary_ip);
    address_from.extend(binary_port);

    let node_id:Vec<u8> =  vec![0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x12];
    let user_agent:&[u8] = "\x0C/bcpc:0.0.1/".as_bytes();
    // let height:u32 =580259;
    let height:u32 = 708998;

    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().write_u32::<LittleEndian>(version).unwrap();
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().write_u64::<LittleEndian>(services).unwrap();
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().write_u64::<LittleEndian>(date_buffer).unwrap();
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().write_u64::<LittleEndian>(address_buffer).unwrap();
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().write_u64::<LittleEndian>(services).unwrap();
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().extend(address_from.clone());
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().write_u64::<LittleEndian>(services).unwrap();
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().extend(address_from.clone());
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().extend(node_id);
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().extend(user_agent);
    TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().write_u32::<LittleEndian>( height).unwrap();

    // let count:Vec<u8> = vec![0x02];
    // let genesis_block:Vec<u8> = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0xd6, 0x68, 0x9c, 0x08, 0x5a, 0xe1, 0x65, 0x83, 0x1e, 0x93, 0x4f, 0xf7, 0x63, 0xae, 0x46, 0xa2, 0xa6, 0xc1, 0x72, 0xb3, 0xf1, 0xb6, 0x0a, 0x8c, 0xe2, 0x6f];
                                    // 00    00    00    00    00    00    00    00    00    02    8b    87    16    d0    52    3f    2f    14    0b    c6    26    49    a7    f9    39    d1    b8    0b    5a    89    c3    a6
    //let genesis_block:Vec<u8> = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x8b, 0x87, 0x16, 0xd0, 0x52, 0x3f, 0x4f, 0x14, 0x0b, 0xc6, 0x26, 0x49, 0xa7, 0xf9, 0x39, 0xd1, 0xb8, 0x0b, 0x5a, 0x89, 0xc3, 0xa6];
    // 00 00 00 00 00 19 d6 68 9c 08 5a e1 65 83 1e 93 4f f7 63 ae 46 a2 a6 c1 72 b3 f1 b6 0a 8c e2 6f

    // let genesis_block:Vec<u8> = vec![0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72, 0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63, 0xf7, 0x4f, 0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c, 0x68, 0xd6, 0x19, 0x00, 0x00, 0x00, 0x00, 0x00];
    // let hash_stop:Vec<u8> =     vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    // let genesis_block:Vec<u8> = Vec::from_hex("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f").unwrap();
    let hash_stop:Vec<u8> =     Vec::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap();

    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().write_u32::<LittleEndian>(version).unwrap();
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(vec![0x15]);
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("0000000000000000000c36d591ad8c0c7330e84458ffde6c6c024d7ae888cffe").unwrap()); //709845
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("000000000000000000055988c1b0ab4440f0f7583056c580e9a0aa6ac8683b57").unwrap()); //709388
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("0000000000000000000387F16D9853CA4CCA63B7BC0AA7FBBB2268DF7FB0B3FD").unwrap()); //709387
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("00000000000000000003A0B28AC8C3BE728FD8446CC68C6C3E1CD53A2A79E034").unwrap()); //709386
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("0000000000000000000a482bf62cd477fd422ef92fa2a7e5e68038ecbbff5775").unwrap()); //709082
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("0000000000000000000883AED93761D3DEECE45AE975D23FE408DA8D2A4EBEDD").unwrap()); //709082
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("0000000000000000000381EB49D93C588D60A377A9BE00A3FAE6827856B7735E").unwrap()); //709081
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("0000000000000000000017E3E40294241F45D1D9FB201A57E296588BF3A129C9").unwrap()); //709077
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("0000000000000000000A66661135CC362AAB9D3A64837BC4A13C76957A0A4CD9").unwrap()); //709076
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("0000000000000000000950D380817357CC6E9425B13098904B8192EAC4849198").unwrap()); //709043
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("000000000000000000DA4BFF4FE47B622A37E51B13B44768C3B013D006FC0173").unwrap()); //460602
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("000000006A625F06636B8BB6AC7B960A8D03705D1ACE08B1A19DA3FDCC99DDBD").unwrap()); //2
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("00000000839A8E6886AB5951D76F411475428AFC90947EE320161BBF18EB6048").unwrap()); //1
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(Vec::from_hex("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f").unwrap()); //0
    TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().extend(hash_stop);

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
