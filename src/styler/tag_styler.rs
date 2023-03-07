use crate::styler::Styler;
use crossterm::style::Stylize;

pub struct TagStyler {}

impl TagStyler {
    pub fn new() -> Self {
        TagStyler {}
    }
}
impl Styler for TagStyler {
    fn style(&self, text: &str, key: &str) -> String {
        let new_line = " NEW_LINE ";
        let formated_text = match key {
            "a" => text.blue().underline(crossterm::style::Color::Blue),
            "p" | "div" => text.red(),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => text.green().bold(),
            "b" => text.white().bold(),
            "em" => text.white().italic(),
            "li" => text.yellow(),
            "dt" | "dd" | "blockquote" | "q" => text.green().italic(),
            "span" => text.white().on_blue().bold(),
            "i" => text.white().italic(),
            "body" | "script" | "head" | "link" | "!DOCTYPE" | "html" | "?xml" => "".white(),
            _ => text.white(),
        };

        formated_text.to_string()
    }
}
