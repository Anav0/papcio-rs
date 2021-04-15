#![allow(dead_code)]
use regex::Regex;
use std::collections::HashMap;
use std::convert::TryInto;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::stdin;
use std::io::stdout;
use std::io::BufRead;
use std::io::Read;
use std::io::Result;
use std::io::Stdin;
use std::io::Stdout;
use std::io::Write;
use std::option::Option::{None, Some};
use std::path::{Path, PathBuf};
use std::{thread, time};
use termion::color::Color;
use termion::cursor;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, style};
use xmltree::Element;
use xmltree::XMLNode::Element as ElementEnum;

extern crate regex;
extern crate xmltree;

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
    toc: Vec<Toc>,
    selected_option: usize,
    terminal_width: (u16, u16),
}

impl<'a> Pupcio<'a> {
    fn new() -> Self {
        Self {
            TMP_FOLDER: "./tmp",
            CONFIG_FOLDER: "./config",
            toc: vec![],
            selected_option: 0,
            terminal_width: termion::terminal_size().unwrap(),
        }
    }

    fn load(&mut self, file_path: &str) -> Result<()> {
        //Extract .epub file
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

        //Get content.opf location
        let container_xml_str = format!("{}/{}", &extract_str, "META-INF/container.xml");
        let mut attrs_to_find = HashMap::new();
        attrs_to_find.insert("rootfile", "full-path");

        let mut content_path = PathBuf::new();
        content_path.push(&extract_str);
        match Regex::new("<rootfile.*full-path=\"(.*?)\".*")
            .unwrap()
            .captures(fs::read_to_string(&container_xml_str)?.as_str())
        {
            Some(captured) => {
                if captured.len() != 2 {
                    panic!("Could't find content.opf location in container.xml")
                }
                content_path.push(&captured[1]);
            }
            None => {
                panic!("Could't find content.opf location in container.xml")
            }
        }

        //Get link to TOC
        let content_file_contents = &fs::read_to_string(&content_path)?;
        let tag = &Regex::new(r"<.*application/x-dtbncx\+xml.*/>")
            .unwrap()
            .captures(content_file_contents)
            .expect("Could't find toc lick in content.opf")[0];

        let toc_path = match Regex::new("<.*href=\"(.*?)\".*/>").unwrap().captures(tag) {
            Some(captures) => captures[1].to_owned(),
            None => {
                panic!("Could't find toc location in content.opf")
            }
        };

        let content_path_ancestors: Vec<_> = content_path.ancestors().collect();
        let toc_path_str = format!(
            "{}/{}",
            content_path_ancestors[1].to_str().unwrap(),
            &toc_path
        );
        let toc_path = Path::new(&toc_path_str);

        if !toc_path.exists() {
            panic!("toc.ncx file doesn't exist")
        }

        //Parse TOC
        //Get TOC nav items
        self.toc = Vec::new();
        {
            let toc_file = File::open(toc_path_str).expect("Cannot open toc.ndx file");
            let toc_tree = Element::parse(toc_file).unwrap();
            for el in &toc_tree
                .get_child("navMap")
                .expect("Couldn't find navMap inside of toc.ndx")
                .children
            {
                match el {
                    ElementEnum(element) => {
                        if element.name == "navPoint" {
                            let text = element
                                .get_child("navLabel")
                                .expect("Cannot find navLabel inside of one of navPoints")
                                .get_child("text")
                                .expect("Cannot find text inside of one of navLabel")
                                .children[0]
                                .as_text()
                                .expect("text tag inside of navLabel do not have content specifed");

                            let src = &element
                                .get_child("content")
                                .expect("Cannot find content inside of navLabel")
                                .attributes["src"];

                            self.toc.push(Toc::new(src.to_owned(), text.to_owned()));
                        }
                    }
                    _ => {}
                }
            }
        }

        {
            self.print_toc();
            let stdin = stdin();
            let mut stdout = stdout().into_raw_mode().unwrap();
            for c in stdin.keys() {
                match c.unwrap() {
                    Key::Char('w') => self.move_selection(MoveDirection::Up),
                    Key::Char('s') => self.move_selection(MoveDirection::Down),
                    Key::Char('q') => break,
                    _ => {}
                }
            }
            write!(stdout, "{}{}", cursor::Goto(1, 1), clear::All);
        }
        //Parse html files
        //Display parsed HTML via stdout
        //TODO: Think about saving/loading epub state
        Ok(())
    }

    fn move_selection(&mut self, direction: MoveDirection) {
        match direction {
            MoveDirection::Up => {
                if self.selected_option != 0 {
                    self.selected_option -= 1
                }
            }
            MoveDirection::Down => {
                if self.selected_option != self.toc.len() - 1 {
                    self.selected_option += 1
                }
            }
        }
        self.print_toc();
    }

    fn update_dimentions(&mut self) {
        self.terminal_width = termion::terminal_size().unwrap();
    }

    fn print_toc(&self) {
        let mut stdout = stdout().into_raw_mode().unwrap();
        write!(
            stdout,
            "{}{}{}",
            clear::All,
            cursor::Goto(1, 1),
            cursor::Hide
        );
        for (i, e) in self.toc.iter().enumerate() {
            let start_cell = (usize::from(self.terminal_width.0) / 2) - (e.text.len() / 2);
            if i == self.selected_option {
                write!(
                    stdout,
                    "{}{}{}{}{}",
                    cursor::Goto(start_cell.try_into().unwrap(), (i + 2).try_into().unwrap()),
                    color::Bg(color::White),
                    style::Bold,
                    e.text,
                    style::Reset,
                )
                .unwrap();
            } else {
                write!(
                    stdout,
                    "{}{}{}",
                    cursor::Goto(start_cell.try_into().unwrap(), (i + 2).try_into().unwrap()),
                    e.text,
                    style::Reset,
                )
                .unwrap();
            }
        }
        stdout.flush().unwrap();
    }
}
enum MoveDirection {
    Up,
    Down,
}
#[derive(Debug)]
struct Toc {
    src: String,
    text: String,
}
impl Toc {
    fn new(src: String, text: String) -> Self {
        Self { src, text }
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
