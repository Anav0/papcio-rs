use crate::config::ReaderConfig;
use crate::html::HtmlToLine;
use crate::misc::{ReaderState, Toc, Zipper};
use crate::styler::Styler;
use crate::styler::TagStyler;
use crate::styler::TocStyler;
use crate::term::{TermSize, Terminal, TermionTerminal};
use regex::Regex;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::fs::File;
use std::io::{stdin, stdout, Write};
use std::option::Option::{None, Some};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use termion::{color, style};
use xmltree::Element;
use xmltree::XMLNode::Element as ElementEnum;

pub struct EpubReader<'a> {
    toc: Vec<Toc>,
    state: ReaderState,
    term: Box<dyn Terminal>,
    config: ReaderConfig<'a>,
    loaded_lines: Vec<String>,
}

impl<'a> EpubReader<'a> {
    pub fn new() -> Self {
        EpubReader {
            toc: vec![],
            state: ReaderState::TocShown,
            term: Box::new(TermionTerminal::new()),
            config: ReaderConfig::new(30, 5, "./tmp"),
            loaded_lines: vec![],
        }
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

        let extract_str = [self.config.tmp_path, epub_file_name].join("/");
        let extract_path = Path::new(&extract_str);

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
        let mut terminal_size = self.term.get_size().expect("Failed to get terminal size");
        let mut toc_screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
        let mut content_screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
        let mut selected_option = 0;
        let mut first_line: u16 = 0;
        let styler = TagStyler::new();
        let toc_styler = TocStyler::new();
        self.print_toc(
            &mut toc_screen,
            selected_option,
            &terminal_size,
            &toc_styler,
        );

        let resize_reciver = self
            .term
            .on_resize()
            .expect("Failed to get reciver on resize channel");

        let input_reciver = self
            .term
            .on_input()
            .expect("Failed to get reciver on input channel");

        loop {
            match resize_reciver.try_recv() {
                Ok(term_size) => {
                    terminal_size = term_size;
                    match self.state {
                        ReaderState::ContentShown => {
                            self.print_section(first_line, &mut content_screen, &terminal_size);
                        }
                        ReaderState::TocShown => {
                            self.print_toc(
                                &mut toc_screen,
                                selected_option,
                                &terminal_size,
                                &toc_styler,
                            );
                        }
                    }
                }
                _ => {}
            }

            match input_reciver.try_recv() {
                Ok(key) => {
                    if key == self.config.keys.up {
                        match self.state {
                            ReaderState::TocShown => {
                                if selected_option != 0 {
                                    selected_option -= 1
                                }
                                self.print_toc(
                                    &mut toc_screen,
                                    selected_option,
                                    &terminal_size,
                                    &toc_styler,
                                );
                            }
                            _ => {}
                        }
                    } else if key == self.config.keys.down {
                        match self.state {
                            ReaderState::TocShown => {
                                if selected_option != self.toc.len() - 1 {
                                    selected_option += 1
                                }
                                self.print_toc(
                                    &mut toc_screen,
                                    selected_option,
                                    &terminal_size,
                                    &toc_styler,
                                );
                            }
                            _ => {}
                        }
                    } else if key == self.config.keys.left {
                        match self.state {
                            ReaderState::ContentShown => {
                                if first_line <= 0 {
                                    continue;
                                }
                                first_line -= terminal_size.height - self.config.margin_y * 2;
                                self.term
                                    .clear(&mut content_screen)
                                    .expect("Failed to clear terminal screen");
                                self.print_section(first_line, &mut content_screen, &terminal_size);
                            }
                            _ => {}
                        }
                    } else if key == self.config.keys.right {
                        match self.state {
                            ReaderState::ContentShown => {
                                if usize::from(terminal_size.height) >= self.loaded_lines.len() {
                                    continue; //We already printed everything in one go
                                }

                                if usize::from(
                                    first_line + terminal_size.height - self.config.margin_y,
                                ) >= self.loaded_lines.len()
                                {
                                    continue; //We already printed everything in one go
                                }

                                first_line += terminal_size.height - self.config.margin_y * 2;
                                self.term
                                    .clear(&mut content_screen)
                                    .expect("Failed to clear terminal screen");
                                self.print_section(first_line, &mut content_screen, &terminal_size);
                            }
                            _ => {}
                        }
                    } else if key == self.config.keys.select {
                        match self.state {
                            ReaderState::TocShown => {
                                self.loaded_lines = HtmlToLine::as_lines(
                                    &self.toc[selected_option].src,
                                    &styler,
                                    terminal_size.width - (self.config.margin_x * 2),
                                );
                                self.state = ReaderState::ContentShown;
                                first_line = 0;
                                self.term
                                    .clear(&mut content_screen)
                                    .expect("Failed to clear terminal screen");
                                self.print_section(first_line, &mut content_screen, &terminal_size);
                            }
                            _ => {}
                        }
                    } else if key == self.config.keys.back {
                        match self.state {
                            ReaderState::ContentShown => {
                                self.term
                                    .clear(&mut content_screen)
                                    .expect("Failed to clear terminal screen");
                                self.state = ReaderState::TocShown;
                                self.print_toc(
                                    &mut toc_screen,
                                    selected_option,
                                    &terminal_size,
                                    &toc_styler,
                                );
                            }
                            _ => {
                                self.term
                                    .clear(&mut content_screen)
                                    .expect("Failed to clear terminal screen");
                                self.term
                                    .clear(&mut toc_screen)
                                    .expect("Failed to clear terminal screen");
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }

            thread::sleep(Duration::from_millis(16));
        }
    }

    pub fn run(&mut self, file_path: &str) {
        self.initialize(file_path)
            .expect("Failed to initialize EpubReader");
        self.listen();
    }

    fn print_section<W: Write>(&self, start_line: u16, screen: &mut W, terminal_size: &TermSize) {
        self.term.clear(screen).unwrap();
        let end_line = start_line + terminal_size.height - self.config.margin_y * 2;
        let lines_to_print = match end_line as usize >= self.loaded_lines.len() {
            true => &self.loaded_lines[start_line as usize..],
            false => &self.loaded_lines[start_line as usize..end_line as usize],
        };

        let mut row = self.config.margin_y;

        for line in lines_to_print {
            self.term
                .write(screen, row, self.config.margin_x, line)
                .expect("Error occured while trying to print lines of text from book's chapter");
            row += 1;
        }
        screen.flush().unwrap();
    }

    fn print_toc<W: Write>(
        &self,
        screen: &mut W,
        selected_option: usize,
        terminal_size: &TermSize,
        styler: &dyn Styler,
    ) {
        self.term.clear(screen).unwrap();
        for (i, e) in self.toc.iter().enumerate() {
            let start_cell = (usize::from(terminal_size.width) / 2) - (e.text.len() / 2);
            if i == selected_option {
                self.term
                    .write(
                        screen,
                        (i + 2).try_into().unwrap(),
                        start_cell.try_into().unwrap(),
                        &styler.style(&e.text, "selected"),
                    )
                    .expect("Problem occured while trying to print table of content");
            } else {
                self.term
                    .write(
                        screen,
                        (i + 2).try_into().unwrap(),
                        start_cell.try_into().unwrap(),
                        &styler.style(&e.text, "not_selected"),
                    )
                    .expect("Problem occured while trying to print table of content");
            }
        }
        screen.flush().unwrap();
    }
}
