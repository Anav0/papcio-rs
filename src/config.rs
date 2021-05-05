pub struct ReaderConfig<'a> {
    pub margin_x: u16,
    pub margin_y: u16,
    pub tmp_path: &'a str,
}

impl<'a> ReaderConfig<'a> {
    pub fn new(margin_x: u16, margin_y: u16, tmp_path: &'a str) -> Self {
        Self {
            margin_y,
            margin_x,
            tmp_path,
        }
    }

    pub fn load_from_file(config_file_path: &str) -> Self {
        todo!()
    }
}
