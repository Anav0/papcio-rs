use crate::styler::Styler;
use termion::{color, style};

pub struct TocStyler {}
impl TocStyler {
    pub fn new() -> Self {
        Self {}
    }
}
impl Styler for TocStyler {
    fn style(&self, text: &str, key: &str) -> std::string::String {
        let formated_text = match key {
            "not_selected" => {
                format!("{}{}{}", style::Bold, text, style::Reset,)
            }
            "selected" => format!(
                "{}{}{}{}",
                color::Bg(color::White),
                style::Bold,
                text,
                style::Reset,
            ),
            _ => text.to_owned(),
        };
        format!("{}{}{}", style::Reset, formated_text, style::Reset)
    }
}
