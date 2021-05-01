use termion::{color, style};

pub trait Styler {
    fn style(&self, text: &str, tag: &str) -> String;
}

pub struct TagStyler {}

impl TagStyler {
    pub fn new() -> Self {
        TagStyler {}
    }
}
impl Styler for TagStyler {
    fn style(&self, text: &str, tag: &str) -> String {
        let new_line = " NEW_LINE ";
        let formated_text = match tag {
            "a" => format!("{}{}{}", color::Fg(color::Blue), style::Underline, text),
            "p" | "div" => format!("{}{}", new_line, text),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                format!("{}{}{}", style::Bold, new_line, text)
            }
            "b" => {
                format!("{}{}", style::Bold, text)
            }
            "em" => format!("{}{}{}", style::Bold, style::Underline, text),
            "li" => format!("{}{}• {}", new_line, color::Fg(color::Yellow), text),
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
            "i" => format!("{}{}", style::Italic, text),
            "body" | "script" | "head" | "link" | "!DOCTYPE" | "html" | "?xml" => String::new(),
            _ => text.to_owned(),
        };
        format!("{}{}{}", style::Reset, formated_text, style::Reset)
    }
}

pub struct EmptyStyler;
impl EmptyStyler {
    pub fn new() -> Self {
        Self {}
    }
}
impl Styler for EmptyStyler {
    fn style(&self, text: &str, tag: &str) -> std::string::String {
        text.to_owned()
    }
}
