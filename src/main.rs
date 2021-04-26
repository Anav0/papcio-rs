#![allow(dead_code)]
use regex::Regex;
use regex::RegexSet;
use std::collections::HashMap;
use std::convert::TryInto;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::stdin;
use std::io::stdout;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Lines;
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

    let mut papcio = Papcio::new();

    papcio.load(file_path)?;

    papcio.run();

    Ok(())
}

struct Papcio<'a> {
    TMP_FOLDER: &'a str,
    CONFIG_FOLDER: &'a str,
    toc: Vec<Toc>,
    selected_option: usize,
    terminal_size: (u16, u16),
}

impl<'a> Papcio<'a> {
    fn new() -> Self {
        Papcio {
            TMP_FOLDER: "./tmp",
            CONFIG_FOLDER: "./config",
            toc: vec![],
            selected_option: 0,
            terminal_size: termion::terminal_size().unwrap(),
        }
    }

    fn run(&mut self) {
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

    fn read_from(&self, toc: &Toc, from: HtmlReadFrom) {
        println!("{}", toc.src);

        let styler = TagStyler::new();
        let paragraph_iterator = HtmlParagraphIterator::new(&toc.src, from, &styler);

        for paragraph in paragraph_iterator {
            println!("{}", paragraph);
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
    }

    fn update_dimentions(&mut self) {
        self.terminal_size = termion::terminal_size().unwrap();
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

struct HtmlParagraphIterator<'a> {
    lines: Lines<BufReader<File>>,
    styler: &'a TagStyler,
    tags: [&'a str; 18],
    regexes: Vec<String>,
}
impl<'a> HtmlParagraphIterator<'a> {
    fn new(filepath: &str, from: HtmlReadFrom, styler: &'a TagStyler) -> Self {
        let html_path = Path::new(filepath);

        if !html_path.exists() {
            panic!("File at: '{}' do not exists", &filepath);
        }

        let html_file =
            File::open(html_path).expect(&format!("File at: '{}' cannot be open", &filepath));

        let mut lines = BufReader::new(html_file).lines();
        let mut skiped = 0;

        match from {
            HtmlReadFrom::Line(line) => {
                loop {
                    skiped += 1;

                    lines
                        .next()
                        .expect(&format!(
                            "Cannot get next '{}' line in file: '{}'",
                            skiped, filepath
                        ))
                        .unwrap();
                    if skiped + 1 >= line {
                        break;
                    }
                }
                skiped
            }
            HtmlReadFrom::Marker(marker) => {
                let marker_re = Regex::new(&format!("^<p.*href=\"{}\".*</p>$", marker)).unwrap();
                loop {
                    skiped += 1;

                    let line = lines
                        .next()
                        .expect(&format!(
                            "Cannot get next '{}' line in file: '{}'. Marker: '{}' not found",
                            skiped, filepath, marker
                        ))
                        .unwrap();

                    let matches = match marker_re.captures(&line) {
                        Some(matches) => matches,
                        None => continue,
                    };

                    if matches.len() > 1 {
                        panic!(format!(
                            "More than one marker: '{}' found i file: '{}",
                            marker, filepath
                        ))
                    }

                    if matches.len() == 1 {
                        break;
                    }
                }
                skiped
            }
        };

        let tags = [
            "p",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "div",
            "a",
            "i",
            "li",
            "em",
            "q",
            "dt",
            "dd",
            "blockquote",
            "b",
            "span",
        ];
        let mut regexes = Vec::with_capacity(tags.len());

        for elem in tags.iter() {
            regexes.push(format!("<{}.*?>(.*?)</{}>", elem, elem));
        }

        Self {
            lines,
            styler,
            regexes,
            tags,
        }
    }
}
impl Iterator for HtmlParagraphIterator<'_> {
    type Item = String;
    fn next(&mut self) -> Option<<Self as std::iter::Iterator>::Item> {
        let regex_set = RegexSet::new(self.regexes.iter()).unwrap();

        loop {
            let line = match self.lines.next() {
                Some(line) => line.unwrap(),
                None => return None,
            };

            let mut tag_content = line.to_owned();
            let mut output = line.to_owned();
            let mut prev_tag = "";
            let mut i = 0;
            loop {
                let total_matches = regex_set
                    .matches(&tag_content)
                    .into_iter()
                    .collect::<Vec<_>>();

                if total_matches.len() == 0 {
                    if i == 0 {
                        tag_content = String::new();
                    }
                    break;
                }
                let regex = &self.regexes[total_matches[0]];
                let tag = &self.tags[total_matches[0]];

                let what_matched = Regex::new(&regex).unwrap().captures(&tag_content).unwrap();

                if what_matched.len() > 2 {
                    panic!("More matched!")
                }

                let whole = what_matched.get(0)?;
                let inner = what_matched.get(1)?.as_str();
                let mut additional_preappend = "";
                if prev_tag != "li" && *tag == "li" {
                    additional_preappend = "\n\r"
                }

                let styled = &self.styler.style(inner, tag, additional_preappend);

                output.replace_range(whole.range(), styled);

                tag_content = output.clone();
                prev_tag = tag;
                i += 1;
            }
            tag_content = tag_content.replace(&self.styler.new_line, "\n\r");
            return Some(tag_content);
        }
    }
}

struct TagStyler {
    new_line: String,
}
impl TagStyler {
    fn new() -> Self {
        TagStyler {
            new_line: String::from("new_line"),
        }
    }
    fn style(&self, text: &str, tag: &str, prepend: &str) -> String {
        let style = match tag {
            "a" => format!("{}{}{}", color::Fg(color::Blue), style::Underline, text),
            "p" | "div" => format!("{}{}", text, self.new_line),
            "h1" | "h2" | "h3" | "h4" | "h6" | "b" => {
                format!("{}{}{}", style::Bold, text, self.new_line)
            }
            "em" => format!("{}{}{}", style::Bold, style::Underline, text),
            "li" => format!("• {}{}{}", color::Fg(color::Yellow), text, self.new_line),
            "dt" | "dd" | "blockquote" | "q" => {
                format!("{}{}", color::Fg(color::Green), text)
            }
            "span" => format!(
                "{}{}{}{}",
                color::Bg(color::White),
                color::Fg(color::Black),
                style::Bold,
                text
            ),
            _ => String::new(),
        };
        format!("{}{}{}", prepend, style, style::Reset)
    }
}

enum HtmlReadFrom {
    Line(usize),
    Marker(String),
}
enum MoveDirection {
    Up,
    Down,
}
#[derive(Debug)]
struct Toc {
    src: String,
    marker: String,
    text: String,
}
impl Toc {
    fn new(src: String, marker: String, text: String) -> Self {
        Self { src, marker, text }
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
