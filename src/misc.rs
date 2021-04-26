use std::fs;
use std::io;
use std::option::Option::{None, Some};
use std::path::{Path, PathBuf};
use std::result::Result;

pub enum MoveDirection {
    Up,
    Down,
}

#[derive(Debug)]
pub struct Toc {
    pub src: String,
    pub marker: String,
    pub text: String,
}
impl Toc {
    pub fn new(src: String, marker: String, text: String) -> Self {
        Self { src, marker, text }
    }
}

pub struct Zipper;
impl<'a> Zipper {
    pub fn unzip(file_path: &str, unzip_location: &Path) -> Result<(), &'a str> {
        let file: fs::File = fs::File::open(file_path).expect("Cannot open zipped file");
        let mut archive = zip::ZipArchive::new(file).expect("Cannot created new zip archive");
        let err_msg = "Problem occured while creating output tmp directory for epub file";
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .expect("Problem occured while iteration over epub sub files");

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
                fs::create_dir_all(&outpath).expect(err_msg);
            } else {
                println!(
                    "File {} extracted to \"{}\" ({} bytes)",
                    i,
                    outpath.display(),
                    file.size()
                );
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p).expect(err_msg);
                    }
                }
                let mut outfile = fs::File::create(&outpath).expect(err_msg);
                io::copy(&mut file, &mut outfile).expect(err_msg);
            }
        }
        Ok(())
    }
}
