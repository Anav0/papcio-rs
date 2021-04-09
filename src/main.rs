#![allow(dead_code)]
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Result;
use std::path::{Path, PathBuf};
use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};

extern crate xml;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("No file path provided")
    }

    let file_path: &String = &args[1];
    println!("{:?}", file_path);

    let mut pupcio = Pupcio::new();

    pupcio.load(file_path)?;

    Ok(())
}

struct Pupcio<'a> {
    TMP_FOLDER: &'a str,
    CONFIG_FOLDER: &'a str,
}

impl<'a> Pupcio<'a> {
    fn new() -> Self {
        Self {
            TMP_FOLDER: "./tmp",
            CONFIG_FOLDER: "./config",
        }
    }

    fn load(&mut self, file_path: &str) -> Result<()> {
        let epub_file_path = Path::new(file_path);

        if !epub_file_path.exists() {
            panic!("File doesn't exists")
        }

        if epub_file_path.is_dir() {
            panic!("File path provided leads to ssomehting other than file")
        }

        let epub_file_name = epub_file_path
            .file_stem()
            .expect("Cannot extract epub file name")
            .to_str()
            .expect("Cannot extract epub file name");

        let extract_str = [self.TMP_FOLDER, epub_file_name].join("/");
        let extract_path = Path::new(&extract_str);

        fs::create_dir_all(self.CONFIG_FOLDER)?;

        if !extract_path.exists() {
            fs::create_dir_all(extract_path)?;
            Zipper::unzip(file_path, &extract_path).expect("Failed to unzip file");
        }

        //Check epub version
        let container_xml_str = format!("{}/{}", extract_str, "META-INF/container.xml");
        let mut attrs_to_find = HashMap::new();
        attrs_to_find.insert("rootfile", "full-path");

        let mut content_path = PathBuf::new();
        content_path.push(extract_str);
        XmlFinder::find(
            Path::new(&container_xml_str),
            &attrs_to_find,
            |attr: &OwnedAttribute| content_path.push(&attr.value),
        )?;

        //Parse book spine
        //Parse TOC
        //Parse html files
        //Display parsed HTML via stdout
        //TODO: Think about saving/loading epub state
        Ok(())
    }
}

struct XmlFinder;
impl<'a> XmlFinder {
    fn find<T>(file_path: &'a Path, attribute_names: &HashMap<&str, &str>, mut f: T) -> Result<()>
    where
        T: FnMut(&OwnedAttribute) -> () + Sized,
    {
        if file_path.is_dir() {
            panic!("file_path is not pointing at xml file");
        }
        if file_path.is_dir() {
            panic!("file_path is not pointing at xml file");
        }
        let file = File::open(file_path)?;
        let parser = EventReader::new(file);
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name,
                    namespace: _,
                    attributes,
                }) => {
                    for attr in attributes {
                        match attribute_names.get_key_value(&name.local_name.as_str()) {
                            Some((.., value)) => {
                                if value == &attr.name.local_name.as_str() {
                                    f(&attr);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
                Err(e) => {
                    panic!(format!(
                        "{}: {}",
                        "Error occured while reading container.xml", e
                    ));
                }
            }
        }
        Ok(())
    }
}
struct Zipper;
impl Zipper {
    fn unzip(file_path: &str, unzip_location: &Path) -> Result<()> {
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
        Ok(())
    }
}
