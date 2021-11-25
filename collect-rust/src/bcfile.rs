use std::io::BufReader;
use serde::Deserialize;
use std::fs::File;
use std::io::{LineWriter, stdout, Write};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    pub static ref LOGGER: Mutex<LineWriter<Box<dyn Write + Send>>> = Mutex::new(LineWriter::new(Box::new(stdout())));
    // pub static ref BLOCKS: Mutex<LineWriter<Box<dyn Write + Send>>> = Mutex::new(LineWriter::new(Box::new(File::create("./blocks.raw").unwrap())));
    pub static ref sortie:LineWriter<File> = LineWriter::new(File::create("./blocks.raw").unwrap());
}

#[derive(Debug, Deserialize)]
pub struct Block {
    pub elem: String,
    pub previous_idx: u32
}

pub fn load_blocks() -> Vec<Block> {
  let file = File::open("./blocks.json").unwrap();
  ::serde_json::from_reader(BufReader::new(file)).unwrap()
}

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
