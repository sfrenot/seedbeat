use std::io::BufReader;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::{LineWriter, stdout, Write};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    pub static ref LOGGER: Mutex<LineWriter<Box<dyn Write + Send>>> = Mutex::new(LineWriter::new(Box::new(stdout())));
    // pub static ref BLOCKS: Mutex<LineWriter<Box<dyn Write + Send>>> = Mutex::new(LineWriter::new(Box::new(File::create("./blocks.raw").unwrap())));
    pub static ref SORTIE:LineWriter<File> = LineWriter::new(File::create("./blocks.raw").unwrap());
}

#[derive(Debug, Deserialize)]
pub struct Block {
    pub elem: String,
    pub next: bool
}

pub fn load_blocks() -> Vec<Block> {
  let file = File::open("./blocks.json").unwrap();
  ::serde_json::from_reader(BufReader::new(file)).unwrap()
}

pub fn store_blocks(blocks: &Vec<(String, bool)>) {
  let mut file = LineWriter::new(File::create("./blocks-found.json").unwrap());
  file.write_all(b"[\n");
  for (blocks, next) in blocks {
      file.write_all(format!("\t {{\"elem\": \"{}\", \"next\": {}}}\n", blocks, next).as_ref());
  }
  file.write_all(b"]");
  drop(file);
  fs::rename("./blocks-found.json", "./blocks.json");
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
