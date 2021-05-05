#![allow(dead_code)]
use std::env;

mod config;
mod html;
mod misc;
mod reader;
mod styler;
mod term;

use reader::EpubReader;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("No file path provided")
    }

    let file_path: &String = &args[1];
    println!("{:?}", file_path);

    let mut papcio = EpubReader::new();

    papcio.run(file_path);

    Ok(())
}
