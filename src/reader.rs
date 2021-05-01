use crate::html::HtmlToLine;
use crate::misc::ReaderState;
use crate::misc::{Toc, Zipper};
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
use termion::screen::AlternateScreen;
use termion::{clear, color, style};
use xmltree::Element;
use xmltree::XMLNode::Element as ElementEnum;

pub struct EpubReader<'a> {
    TMP_FOLDER: &'a str,
    CONFIG_FOLDER: &'a str,
    toc: Vec<Toc>,
    terminal_width: u16,
    terminal_height: u16,
    state: ReaderState,
    margin_x: u16,
    margin_y: u16,
    loaded_lines: Vec<String>,
}

impl<'a> EpubReader<'a> {
    pub fn new() -> Self {
        let terminal_size = termion::terminal_size().unwrap();

        EpubReader {
            loaded_lines: vec![],
            TMP_FOLDER: "./tmp",
            CONFIG_FOLDER: "./config",
            toc: vec![],
            terminal_height: terminal_size.1,
            terminal_width: terminal_size.0,
            state: ReaderState::TocShown,
            margin_x: 8,
            margin_y: 5,
        }
    }

    fn clear_screen<W: Write>(&self, screen: &mut W) {
        write!(screen, "{}{}", cursor::Goto(1, 1), clear::All);
        screen.flush().unwrap();
    }

    fn clean<W: Write>(&self, screen: &mut W) {
        write!(
            screen,
            "{}{}{}",
            cursor::Goto(1, 1),
            clear::All,
            cursor::Show
        );
        screen.flush().unwrap();
    }

    fn initialize(&mut self, file_path: &str) -> Result<(), &str> {
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

    fn listen(&mut self) {
        let mut toc_screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
        let mut content_screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
        self.update_dimentions();
        let stdin = stdin();
        let mut selected_option = 0;
        let mut first_line: u16 = 0;
        self.print_toc(&mut toc_screen, selected_option);
        let MIN_WIDTH = 80;
        let styler = TagStyler::new();

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('w') => match self.state {
                    ReaderState::TocShown => {
                        if selected_option != 0 {
                            selected_option -= 1
                        }
                        self.print_toc(&mut toc_screen, selected_option);
                    }
                    _ => {}
                },
                Key::Char('s') => match self.state {
                    ReaderState::TocShown => {
                        if selected_option != self.toc.len() - 1 {
                            selected_option += 1
                        }
                        self.print_toc(&mut toc_screen, selected_option);
                    }
                    _ => {}
                },
                Key::Char('a') => match self.state {
                    ReaderState::ContentShown => {
                        if first_line <= 0 {
                            continue;
                        }
                        first_line -= self.terminal_height - self.margin_y * 2;
                        self.clear_screen(&mut content_screen);
                        self.print_section(first_line, &mut content_screen);
                    }
                    _ => {}
                },
                Key::Char('d') => match self.state {
                    ReaderState::ContentShown => {
                        if usize::from(self.terminal_height) >= self.loaded_lines.len() {
                            continue; //We already printed everything in one go
                        }

                        if usize::from(first_line + self.terminal_height - self.margin_y)
                            >= self.loaded_lines.len()
                        {
                            continue; //We already printed everything in one go
                        }

                        first_line += self.terminal_height - self.margin_y * 2;
                        self.clear_screen(&mut content_screen);
                        self.print_section(first_line, &mut content_screen);
                    }
                    _ => {}
                },
                Key::Char('e') => match self.state {
                    ReaderState::TocShown => {
                        self.loaded_lines = HtmlToLine::as_lines(
                            &self.toc[selected_option].src,
                            &styler,
                            MIN_WIDTH,
                            self.margin_x,
                        );
                        self.state = ReaderState::ContentShown;
                        first_line = 0;
                        self.clear_screen(&mut content_screen);
                        self.print_section(first_line, &mut content_screen);
                    }
                    _ => {}
                },
                Key::Char('q') => match self.state {
                    ReaderState::ContentShown => {
                        self.clear_screen(&mut content_screen);
                        self.state = ReaderState::TocShown;
                        self.print_toc(&mut toc_screen, selected_option);
                    }
                    _ => {
                        self.clear_screen(&mut content_screen);
                        self.clear_screen(&mut toc_screen);
                        break;
                    }
                },
                _ => {}
            }
        }

        self.clean(&mut content_screen);
        self.clean(&mut toc_screen);
    }

    pub fn run(&mut self, file_path: &str) {
        self.initialize(file_path)
            .expect("Failed to initialize EpubReader");
        self.listen();
    }

    fn print_section<W: Write>(&self, start_line: u16, screen: &mut W) {
        let MIN_HEIGHT = 50;
        let end_line = match self.terminal_height < MIN_HEIGHT {
            true => MIN_HEIGHT,
            false => start_line + self.terminal_height - self.margin_y * 2,
        };
        let lines_to_print = match end_line as usize >= self.loaded_lines.len() {
            true => &self.loaded_lines[start_line as usize..],
            false => &self.loaded_lines[start_line as usize..end_line as usize],
        };

        let mut row = self.margin_y;
        for line in lines_to_print {
            write!(screen, "{}{}", cursor::Goto(self.margin_x, row), line);
            row += 1;
        }
        screen.flush().unwrap();
    }

    fn update_dimentions(&mut self) {
        let terminal_size = termion::terminal_size().unwrap();
        self.terminal_height = terminal_size.1;
        self.terminal_width = terminal_size.0;
    }

    fn print_toc<W: Write>(&self, screen: &mut W, selected_option: usize) {
        for (i, e) in self.toc.iter().enumerate() {
            let start_cell = (usize::from(self.terminal_width) / 2) - (e.text.len() / 2);
            if i == selected_option {
                write!(
                    screen,
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
                    screen,
                    "{}{}{}",
                    cursor::Goto(start_cell.try_into().unwrap(), (i + 2).try_into().unwrap()),
                    e.text,
                    style::Reset,
                )
                .unwrap();
            }
        }
        screen.flush().unwrap();
    }
}