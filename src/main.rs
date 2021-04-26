#![allow(dead_code)]
use std::env;

extern crate regex;
extern crate xmltree;

mod html;
mod misc;
mod papcio;
mod styler;

use papcio::Papcio;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("No file path provided")
    }

    let file_path: &String = &args[1];
    println!("{:?}", file_path);

    let mut papcio = Papcio::new();

    papcio.run(file_path);

    Ok(())
}
