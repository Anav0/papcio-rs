#![allow(dead_code)]
use std::env;
use std::fs;
use std::io;
use std::io::Result;
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    const TMP_FOLDER: &str = "./tmp";
    const CONFIG_FOLDER: &str = "./config";

    let args: Vec<String> = env::args().collect();

    fs::create_dir_all(TMP_FOLDER)?;
    fs::create_dir_all(CONFIG_FOLDER)?;
    fs::remove_dir_all(TMP_FOLDER).expect("TMP folder does not exist");

    if args.len() < 2 {
        panic!("No file path provided")
    }

    let file_path: &String = &args[1];
    println!("{:?}", file_path);

    Zipper::unzip(file_path, TMP_FOLDER).expect("Failed to unzip file");

    Ok(())
}

struct Zipper;
impl Zipper {
    fn unzip(file_path: &str, unzip_location: &str) -> Result<bool> {
        println!("{}", file_path);
        let file: fs::File = fs::File::open(file_path).expect("Cannot open zipped file");

        let mut archive = zip::ZipArchive::new(file).expect("Cannot created new zip archive");

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;

            let outpath = match file.enclosed_name() {
                Some(path) => PathBuf::from(unzip_location).join(path),
                None => continue,
            };
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }

            if (&*file.name()).ends_with('/') {
                println!("File {} extracted to \"{}\"", i, outpath.display());
                fs::create_dir_all(&outpath)?;
            } else {
                println!(
                    "File {} extracted to \"{}\" ({} bytes)",
                    i,
                    outpath.display(),
                    file.size()
                );
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p)?
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }
        Ok(true)
    }
}
