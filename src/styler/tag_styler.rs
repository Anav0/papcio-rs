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
            "a" => text.blue().underline(crossterm::style::Color::Black),
            "p" | "div" => text.black(),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => text.black().bold(),
            "b" => text.black().bold(),
            "em" => text.black().italic(),
            "li" => text.yellow(),
            "dt" | "dd" | "blockquote" | "q" => text.green().italic(),
            "span" => text.black().on_blue().bold(),
            "i" => text.italic(),
            "body" | "script" | "head" | "link" | "!DOCTYPE" | "html" | "?xml" => "".black(),
            _ => text.black(),
        };
        formated_text.reset().to_string()
    }
}
