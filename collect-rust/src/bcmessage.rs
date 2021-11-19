use std::sync::Mutex;
use lazy_static::lazy_static;
use std::time::SystemTime;
use chrono::{DateTime, NaiveDateTime, Utc};
use sha2::{Sha256, Digest};
use std::net::{TcpStream, IpAddr};
use std::io::{Write, Read, Error, ErrorKind};
use std::convert::TryInto;
use hex::FromHex;
use crate::bcblocks;

pub const VERSION:u32 = 70015;
const PORT:u16 = 8333;

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

// Timer
const MESSAGE_TIMEOUT:std::time::Duration = std::time::Duration::from_secs(120);

// services
const NODE_NETWORK:u64 = 1;
const NODE_BLOOM:u64 = 4;
const NODE_WITNESS:u64 = 8;
const NODE_NETWORK_LIMITED:u64  = 1024;

// offset for addr cmd
pub const ADDRESS_LEN: usize =30;
pub const TIME_FIELD_END: usize =4;
pub const SERVICES_END:usize = 12;
pub const IP_FIELD_END:usize= 28;
pub const PORT_FIELD_END:usize= 30;

// offset for version cmd
const VERSION_END: usize =4;

const USER_AGENT: usize = 80;
const TIMESTAMP_END:usize= 20;

// payload struct
lazy_static! {
    static ref TEMPLATE_MESSAGE_PAYLOAD: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(105));
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
pub const GET_HEADERS:&str = "getheaders";
pub const HEADERS:&str = "headers";

// pub const GET_BLOCKS:&str = "getblocks";
pub const GET_DATA:&str = "getdata";
pub const BLOCK: &str = "block";

pub struct ReadResult {
    pub command: String,
    pub payload: Vec<u8>,
    pub error: Option<std::io::Error>
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

    message_payload.extend(VERSION.to_le_bytes());
    message_payload.extend(services.to_le_bytes());
    message_payload.extend(date_buffer.to_le_bytes());
    message_payload.extend(address_buffer.to_le_bytes());
    message_payload.extend(services.to_le_bytes());
    message_payload.extend(&address_from);
    message_payload.extend(services.to_le_bytes());
    message_payload.extend(&address_from);
    message_payload.extend(node_id);
    message_payload.extend(user_agent);
    message_payload.extend(height.to_le_bytes());

    // drop(message_payload);
    // eprintln!("{:02x?}", hex::encode(TEMPLATE_MESSAGE_PAYLOAD.lock().unwrap().to_vec()));
    // std::process::exit(1);
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

            let payload_size = u32::from_le_bytes((&header_buffer[START_PAYLOAD_LENGTH..END_PAYLOAD_LENGTH]).try_into().unwrap());

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


fn build_request(message : &str) -> Vec<u8>{
    let mut payload_bytes: Vec<u8> = Vec::new();
    let mut message_name = message;
    if message == MSG_VERSION {
        payload_bytes = get_payload_with_current_date();
        // eprintln!("->MSG_VERSION : {:02X?}", payload_bytes);
    // } else if message == GET_BLOCKS {
    //     payload_bytes = bcblocks::get_getblock_message_payload();
    } else if message == GET_HEADERS {
        // payload_bytes = TEMPLATE_GETBLOCK_PAYLOAD.lock().unwrap().clone();
        payload_bytes = bcblocks::get_getheaders_message_payload();
        // eprintln!("==> GET_BLOCKS : {:02X?}", payload_bytes);
        // std::process::exit(1);
    } else if message.len() > GET_DATA.len() && &message[..GET_DATA.len()] == String::from(GET_DATA) {

        payload_bytes = bcblocks::get_getdata_message_payload(&message[GET_DATA.len()+1..]);
        message_name = GET_DATA;
        println!("Build for getData : {}", message_name);

        // eprintln!("Build for getData : {:02x?}", payload_bytes);
        // std::process::exit(1);
    }
    // eprintln!("{} <-> {}", &message, &message_name);

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
    date.extend(unix_timestamp.swap_bytes().to_be_bytes());
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
    payload_len_buffer.extend(payload_len.swap_bytes().to_be_bytes());
    let slice:&[u8] =&payload_len_buffer[..];
    header.splice(START_PAYLOAD_LENGTH..END_PAYLOAD_LENGTH, slice.iter().cloned());

    let checksum = compute_checksum(payload);
    header.splice(START_CHECKSUM..END_CHECKSUM, checksum.iter().cloned());
}

pub fn process_version_message(payload: &Vec<u8>) -> (u32, Vec<u8>, DateTime<Utc>, String) {

    let version_number = u32::from_le_bytes((&payload[..VERSION_END]).try_into().unwrap());
    let services = payload[VERSION_END..SERVICES_END].to_vec();
    let peer_time = get_date_time(payload[SERVICES_END..TIMESTAMP_END].to_vec());

    let useragent_size = get_compact_int(&payload[USER_AGENT..].to_vec()) as usize;
    let start_byte= get_start_byte(&useragent_size);

    let mut user_agent = String::new();
    if USER_AGENT + start_byte + useragent_size < payload.len() {
        if useragent_size > 0 {
            let user_agent_slice = &payload[(USER_AGENT + start_byte)..(USER_AGENT + start_byte + useragent_size)];
            user_agent.push_str(String::from_utf8(user_agent_slice.to_vec()).unwrap().as_str() );

        }
    }
    (version_number, services, peer_time, user_agent)
}

pub fn process_addr_message(payload: Vec<u8>) -> Vec<String>{
    if payload.len() == 0 {
        return vec![];
    }
    let addr_number = get_compact_int(&payload);
    if addr_number < 2 {
        return vec![];
    }

    let mut addr = vec![];
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

        let port = u16::from_be_bytes((&payload[addr_begins_at+ IP_FIELD_END..addr_begins_at+ PORT_FIELD_END]).try_into().unwrap());
        let new_peer: String = format!("{}:{:?}", ip_v4, port);

        addr.push(new_peer);
        read_addr = read_addr +1;
    }
    // eprintln!("--> Ajout {} noeuds", new_addr);
    addr
}

//// COMMON SERVICES
fn get_compact_int(payload: &Vec<u8>) -> u64 {
    let storage_length: u8 = payload[STORAGE_BYTE];
    // TODO: Try with match construct
    if storage_length == UNIT_16 {
        return u16::from_le_bytes((&payload[NUM_START..UNIT_16_END]).try_into().unwrap()) as u64;
    }
    if storage_length == UNIT_32 {
        return u32::from_le_bytes((&payload[NUM_START..UNIT_32_END]).try_into().unwrap()) as u64;
    }
    if storage_length == UNIT_64 {
        return u64::from_le_bytes((&payload[NUM_START..UNIT_64_END]).try_into().unwrap()) as u64;
    }
    return storage_length as u64;
}

fn get_date_time(mut time_vec: Vec<u8>) -> DateTime<Utc>{
    if time_vec.len() == 4 {
        /* La taille du champ varie dans le protocole de 4 à 8 octets */
        time_vec.append(&mut vec![0,0,0,0]);
    }
    return DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(i64::from_le_bytes(time_vec.try_into().unwrap()), 0), Utc);
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
