// mod bcmessage;
mod bcblocks;
mod bcfile;
mod bcnet;
mod bcpeers;

use clap::{Arg, App};
use std::sync::mpsc;
use std::sync::atomic::Ordering;

use std::thread;
use std::process;

use std::time::{Duration, SystemTime};

const CHECK_TERMINATION_TIMEOUT:Duration = Duration::from_secs(5);
const THREADS: u64 = 500;
const MESSAGE_CHANNEL_SIZE: usize = 100000;


fn main() {
    bcfile::load_blocks();
    bcblocks::create_block_message_payload(&bcblocks::BLOCKS_ID.lock().unwrap());

    // eprintln!("{}", hex::encode(bcblocks::get_getblock_message_payload()));
    // eprintln!("{}", hex::encode(bcblocks::get_getheaders_message_payload()));
    // std::process::exit(1);

    // eprintln!("{:?}", known_block);
    // eprintln!("{:?}", bcblocks::BLOCKS_ID.lock().unwrap());
    // std::process::exit(1);

    let (address_channel_sender, address_channel_receiver) = mpsc::channel();
    let (connecting_start_channel_sender, connecting_start_channel_receiver) = chan::sync(MESSAGE_CHANNEL_SIZE);

    let start_adress = parse_args();
    address_channel_sender.send(start_adress).unwrap();
    thread::spawn(move || { check_pool_size(SystemTime::now()); });

    for i in 0..THREADS {
        let sender = address_channel_sender.clone();
        let recv = connecting_start_channel_receiver.clone();
        thread::spawn(move || { bcnet::handle_one_peer(recv, sender, i);});
    }

    loop {
        let new_peer: String = address_channel_receiver.recv().unwrap();
        connecting_start_channel_sender.send(new_peer);
        bcpeers::NB_ADDR_TO_TEST.fetch_add(1, Ordering::Relaxed);
    }
}

fn check_pool_size(start_time: SystemTime ){
    loop {
        thread::sleep(CHECK_TERMINATION_TIMEOUT);

        bcpeers::get_peers_status();
        if bcpeers::NB_ADDR_TO_TEST.load(Ordering::Relaxed) < 1 {
            let time_spent = SystemTime::now().duration_since(start_time).unwrap_or_default();
            println!("POOL Crawling ends in {:?} ", time_spent);
            process::exit(0);
        }
    }
}

fn parse_args() -> String {
    let matches = App::new("BC crawl")
        .version("1.0.0")
        .author("Jazmin Ferreiro  <jazminsofiaf@gmail.com>, Stephane Frenot <stephane.frenot@insa-lyon.fr>")
        .arg(Arg::with_name("file")
            .short("-o")
            .long("output")
            .takes_value(true)
            .required(true)
            .help("output file name for crawl"))
        .arg(Arg::with_name("address")
            .short("-s")
            .long("address")
            .takes_value(true)
            .required(true)
            .help(" Initial address for crawling. Format [a.b.c.d]:ppp"))
        .get_matches();

    let arg_address = matches.value_of("address").unwrap_or_else(|| {
        panic!("Error parsing address argument");
        }
    );

    bcfile::open_logfile(matches.value_of("file"));
    String::from(arg_address)
}
