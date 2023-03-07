#![allow(dead_code)]
use std::env;

mod config;
mod html;
mod misc;
mod reader;
mod styler;
mod term;

use reader::EpubReader;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("No file path provided");
        std::process::exit(0);
    }

    let file_path: &String = &args[1];

    let mut papcio = EpubReader::new();

    match papcio.run(file_path) {
        Ok(()) => {}
        Err(msg) => {
            println!("{}", msg);
            std::process::exit(0);
        }
    }

    Ok(())
}
