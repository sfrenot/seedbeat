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

pub fn store_blocks(blocks: &Vec<(String, bool)>) -> Vec<(String, bool)> {
  let mut file = LineWriter::new(File::create("./blocks-found.json").unwrap());
  let mut res: Vec<(String, bool)>=[].to_vec();
  file.write_all(b"[\n").unwrap();
  for i in 1..blocks.len() {
      let (block, next) = &blocks[i];
      file.write_all(format!("\t {{\"elem\": \"{}\", \"next\": {}}}", block, next).as_ref()).unwrap();
      if i < blocks.len()-1 {
         file.write_all(b",\n").unwrap();
     } else {
         file.write_all(b"\n").unwrap();
     }
      if !next {
          res.push((block.clone(), next.clone()));
      }
  }
  file.write_all(b"]").unwrap();
  drop(file);
  fs::rename("./blocks-found.json", "./blocks.json").unwrap();
  res
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
