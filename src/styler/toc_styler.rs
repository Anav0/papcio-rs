use crossterm::style::Stylize;

use crate::styler::Styler;

pub struct TocStyler {}
impl TocStyler {
    pub fn new() -> Self {
        Self {}
    }
}
impl Styler for TocStyler {
    fn style(&self, text: &str, key: &str) -> String {
        let formated_text = match key {
            "selected" => text.black().on_blue(),
            _ => text.white(),
        };
        formated_text.bold().to_string()
    }
}
