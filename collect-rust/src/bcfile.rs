use std::io::BufReader;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::{LineWriter, stdout, Write};
use lazy_static::lazy_static;
use std::sync::Mutex;
use crate::bcblocks;
use crate::bcmessage;

lazy_static! {
    pub static ref LOGGER: Mutex<LineWriter<Box<dyn Write + Send>>> = Mutex::new(LineWriter::new(Box::new(stdout())));
    // pub static ref BLOCKS: Mutex<LineWriter<Box<dyn Write + Send>>> = Mutex::new(LineWriter::new(Box::new(File::create("./blocks.raw").unwrap())));
    pub static ref SORTIE:LineWriter<File> = LineWriter::new(File::create("./blocks.raw").unwrap());
}

/// Block storage
#[derive(Debug, Deserialize)]
pub struct Block {
    pub elem: String,
    pub next: bool
}

pub fn load_blocks() {
    eprintln!("Début lecture fichier blocks");
    let file = File::open("./blocks.json").unwrap();
    let blocks: Vec<Block> = ::serde_json::from_reader(BufReader::new(file)).unwrap();

    let mut known_block = bcblocks::KNOWN_BLOCK.lock().unwrap();
    let mut blocks_id = bcblocks::BLOCKS_ID.lock().unwrap();

    let mut idx:usize = 1;
    let mut previous: String = "".to_string();
    for item in blocks {
        // eprintln!("-> {}", item.elem);
        blocks_id.push((item.elem.clone(), item.next));
        known_block.insert(item.elem.clone(), bcblocks::BlockDesc{idx, previous});
        if item.next {
            previous = item.elem;
        } else {
            previous = "".to_string();
        }
        idx+=1;
    }
    eprintln!("Fin lecture fichier blocks");
}

pub fn store_blocks(blocks: &Vec<(String, bool)>) -> bool {
    let mut file = LineWriter::new(File::create("./blocks-found.json").unwrap());
    let mut new_blocks = false;
    file.write_all(b"[\n").unwrap();
    for i in 1..blocks.len() {
        let (block, next) = &blocks[i];
        file.write_all(format!("\t {{\"elem\": \"{}\", \"next\": {}}}", block, next).as_ref()).unwrap();
        if i < blocks.len()-1 {
         file.write_all(b",\n").unwrap();
        } else {
         file.write_all(b"\n").unwrap();
        }
        if !new_blocks && !next {
            new_blocks = true;
        }
    }
    file.write_all(b"]").unwrap();
    drop(file);
    fs::rename("./blocks-found.json", "./blocks.json").unwrap();
    new_blocks
}

/// Addr storage
pub fn open_logfile(arg_file: Option<&str>) {
    let file: File;
    match arg_file {
        None => panic!("Error parsing file name"),
        Some(f) =>  {
            file = File::create(f).unwrap();
        }
    }
    let mut logger = LOGGER.lock().unwrap();
    *logger = LineWriter::new(Box::new(file));
}

pub fn store_event(msg :&String){
    let mut guard = LOGGER.lock().unwrap();
    guard.write_all(msg.as_ref()).expect("error at logging");
}

pub fn store_version_message(target_address: String, payload: &Vec<u8>){
    //TODO: supprimer le &VEc
    let (_, _, _, _) = bcmessage::process_version_message(payload);
    let mut msg: String  = String::new();
    msg.push_str(format!("Seed: {} \n", target_address).as_ref());
    // msg.push_str(format!("Seed = {}  ", target_address).as_ref());
    // msg.push_str(format!("version = {}   ", version_number).as_str());
    // msg.push_str(format!("user agent = {}   ", user_agent).as_str());
    // msg.push_str(format!("time = {}  ", peer_time.format("%Y-%m-%d %H:%M:%S")).as_str());
    // msg.push_str(format!("now = {}  ", Into::<DateTime<Utc>>::into(SystemTime::now()).format("%Y-%m-%d %H:%M:%S")).as_str());
    // msg.push_str(format!("since = {:?}  ",SystemTime::now().duration_since(SystemTime::from(peer_time)).unwrap_or_default() ).as_str());
    // msg.push_str(format!("services = {:?}\n", services ).as_str());
    store_event(&msg);
}
