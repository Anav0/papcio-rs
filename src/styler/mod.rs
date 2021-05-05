mod tag_styler;
mod toc_styler;

pub use tag_styler::TagStyler;
pub use toc_styler::TocStyler;

pub trait Styler {
    fn style(&self, text: &str, key: &str) -> String;
}

pub struct EmptyStyler;
impl EmptyStyler {
    pub fn new() -> Self {
        Self {}
    }
}
impl Styler for EmptyStyler {
    fn style(&self, text: &str, key: &str) -> std::string::String {
        text.to_owned()
    }
}
