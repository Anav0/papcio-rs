use crate::styler::Styler;
use crate::styler::TagStyler;
use regex::Regex;
use regex::RegexSet;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

pub struct HtmlToLine<'a> {
    styler: &'a TagStyler,
}

impl<'a> HtmlToLine<'a> {
    pub fn as_lines<S: Styler>(
        filepath: &str,
        styler: &'a S,
        width: u16,
        margin_x: u16,
    ) -> Vec<String> {
        let html_path = Path::new(filepath);

        if !html_path.exists() {
            panic!("File at: '{}' do not exists", &filepath);
        }

        let html_file =
            File::open(html_path).expect(&format!("File at: '{}' cannot be open", &filepath));

        let lines = BufReader::new(html_file).lines();
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

        let regex_set = RegexSet::new(regexes.iter()).unwrap();
        let mut extracted_lines: Vec<String> = vec![];

        for line in lines {
            let line = line.unwrap();
            let mut tag_content = line.to_owned();
            let mut output = line.to_owned();
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
                let regex = &regexes[total_matches[0]];
                let tag = &tags[total_matches[0]];

                let what_matched = Regex::new(&regex).unwrap().captures(&tag_content).unwrap();

                if what_matched.len() > 2 {
                    panic!("More matched!")
                }

                let whole = what_matched.get(0).unwrap();
                let inner = what_matched.get(1).unwrap().as_str();

                let styled = &styler.style(inner, tag);

                output.replace_range(whole.range(), styled);

                tag_content = output.clone();
                i += 1;
            }

            //TODO: change this to something better
            let max_chars_in_line = (width - margin_x * 2) as usize;
            let words = tag_content.split(" ").collect::<Vec<_>>();
            let mut char_counter = 0;
            let mut tmp_words: Vec<&str> = vec![];
            for word in words {
                match word {
                    "NEW_LINE" => {
                        if !tmp_words.is_empty() {
                            if !extracted_lines.is_empty() {
                                let mut last = extracted_lines.pop().unwrap();
                                last.push_str(tmp_words.join(" ").as_str());
                                extracted_lines.push(last);
                                tmp_words.clear();
                                char_counter = 0;
                            }
                        }
                        extracted_lines.push("".to_owned());
                        continue;
                    }
                    _ => {
                        char_counter += word.chars().count();
                        tmp_words.push(word);
                        if char_counter >= max_chars_in_line {
                            extracted_lines.push(tmp_words.join(" "));
                            tmp_words.clear();
                            char_counter = 0;
                        }
                    }
                }
            }
            if char_counter != 0 {
                extracted_lines.push(tmp_words.join(" "));
            }
        }
        extracted_lines
    }
}

pub enum HtmlReadFrom {
    Line(usize),
    Marker(String),
}

mod tests {
    #[test]
    fn parsing_html_works() {
        use crate::html::HtmlToLine;
        use crate::styler::EmptyStyler;

        let expected_lines = vec![
            "ABC",
            "D EF",
            "PLACEK",
            "",
            "bleblebleblebleble",
            "",
            "AB",
            "CD",
            "ABCD",
            "A B",
            "C D",
            "ABCD",
        ];
        let file_path = "./test_data/test_file.html";
        let styler = EmptyStyler::new();
        let lines = HtmlToLine::as_lines(file_path, &styler, 2, 0);

        assert_eq!(expected_lines.len(), lines.len());

        for (i, expected_line) in expected_lines.iter().enumerate() {
            assert_eq!(expected_line, &lines[i]);
        }
    }
}
