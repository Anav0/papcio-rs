use termion::{color, style};

pub struct TagStyler {
    pub new_line: String,
}
impl TagStyler {
    pub fn new() -> Self {
        TagStyler {
            new_line: String::from("new_line"),
        }
    }
    pub fn style(&self, text: &str, tag: &str, prepend: &str) -> String {
        let style = match tag {
            "a" => format!("{}{}{}", color::Fg(color::Blue), style::Underline, text),
            "p" | "div" => format!("{}{}{}", text, self.new_line, self.new_line),
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
