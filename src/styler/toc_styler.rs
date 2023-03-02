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
            "not_selected" => text.bold().reset(),
            "selected" => text.on_white().bold().reset(),
            _ => text.black(),
        };
        formated_text.reset().to_string()
    }
}
