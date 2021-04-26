use crate::html::HtmlParagraphIterator;
use crate::html::HtmlReadFrom;
use crate::misc::{MoveDirection, Toc, Zipper};
use crate::styler::TagStyler;
use regex::Regex;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::fs::File;
use std::io::stdin;
use std::io::stdout;
use std::io::Write;
use std::option::Option::{None, Some};
use std::path::{Path, PathBuf};
use termion::cursor;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, style};
use xmltree::Element;
use xmltree::XMLNode::Element as ElementEnum;

pub struct Papcio<'a> {
    TMP_FOLDER: &'a str,
    CONFIG_FOLDER: &'a str,
    toc: Vec<Toc>,
    selected_option: usize,
    terminal_size: (u16, u16),
}

impl<'a> Papcio<'a> {
    pub fn new() -> Self {
        Papcio {
            TMP_FOLDER: "./tmp",
            CONFIG_FOLDER: "./config",
            toc: vec![],
            selected_option: 0,
            terminal_size: termion::terminal_size().unwrap(),
        }
    }

    pub fn run(&mut self) {
        self.read_from(&self.toc[3], HtmlReadFrom::Line(1));
        todo!();
        self.update_dimentions();
        self.print_toc();
        let stdin = stdin();
        let mut stdout = stdout().into_raw_mode().unwrap();
        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('w') => self.move_selection(MoveDirection::Up),
                Key::Char('s') => self.move_selection(MoveDirection::Down),
                Key::Char('e') => {
                    let selected_chapter = &self.toc[self.selected_option];
                    self.read_from(selected_chapter, HtmlReadFrom::Line(1));
                }
                Key::Char('q') => break,
                _ => {}
            }
            self.print_toc();
        }
        write!(
            stdout,
            "{}{}{}",
            cursor::Goto(1, 1),
            clear::All,
            cursor::Show
        );
    }

    pub fn read_from(&self, toc: &Toc, from: HtmlReadFrom) {
        println!("{}", toc.src);

        let styler = TagStyler::new();
        let paragraph_iterator = HtmlParagraphIterator::new(&toc.src, from, &styler);

        for paragraph in paragraph_iterator {
            println!("{}", paragraph);
        }
    }

    pub fn load(&mut self, file_path: &str) -> Result<(), &str> {
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

        fs::create_dir_all(self.CONFIG_FOLDER).expect(&format!(
            "Failed to create config folder at: {}",
            self.CONFIG_FOLDER
        ));

        if !extract_path.exists() {
            fs::create_dir_all(extract_path).expect("Failed to tmpfolder at");
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
            .captures(
                fs::read_to_string(&container_xml_str)
                    .expect("Failed to read container xml file")
                    .as_str(),
            ) {
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
        let content_file_contents =
            &fs::read_to_string(&content_path).expect("Failed to read toc file content");
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
            let toc_file = File::open(&toc_path_str).expect("Cannot open toc.ndx file");
            let toc_tree = Element::parse(toc_file).unwrap();

            let mut up_to_toc = PathBuf::from(toc_path);
            up_to_toc.pop();

            let up_to_toc = up_to_toc.to_str().unwrap();

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

                            let split: Vec<&str> = src.split("#").collect();

                            match split.len() {
                                2 => {
                                    self.toc.push(Toc::new(
                                        format!("{}/{}", up_to_toc, split[0]),
                                        split[1].to_owned(),
                                        text.to_owned(),
                                    ));
                                }
                                _ => {
                                    self.toc.push(Toc::new(
                                        format!("{}/{}", up_to_toc, split[0]),
                                        String::from(""),
                                        text.to_owned(),
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        //Parse html files
        //Display parsed HTML via stdout
        //TODO: Think about saving/loading epub state
        Ok(())
    }

    pub fn move_selection(&mut self, direction: MoveDirection) {
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
    }

    pub fn update_dimentions(&mut self) {
        self.terminal_size = termion::terminal_size().unwrap();
    }

    pub fn print_toc(&self) {
        let mut stdout = stdout().into_raw_mode().unwrap();
        write!(
            stdout,
            "{}{}{}",
            clear::All,
            cursor::Goto(1, 1),
            cursor::Hide
        );
        for (i, e) in self.toc.iter().enumerate() {
            let start_cell = (usize::from(self.terminal_size.0) / 2) - (e.text.len() / 2);
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
