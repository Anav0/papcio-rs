use crate::styler::TagStyler;
use regex::Regex;
use regex::RegexSet;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Lines;
use std::option::Option::{None, Some};
use std::path::Path;

pub struct HtmlParagraphIterator<'a> {
    pub lines: Lines<BufReader<File>>,
    pub styler: &'a TagStyler,
    pub tags: [&'a str; 18],
    pub regexes: Vec<String>,
}
impl<'a> HtmlParagraphIterator<'a> {
    pub fn new(filepath: &str, from: HtmlReadFrom, styler: &'a TagStyler) -> Self {
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

pub enum HtmlReadFrom {
    Line(usize),
    Marker(String),
}
