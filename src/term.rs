use std::fmt::Display;
use std::io::{Error, Write};
use termion::{clear, cursor};

pub struct TermSize {
    pub width: u16,
    pub height: u16,
}

pub trait Terminal {
    fn write(
        &self,
        screen: &mut dyn Write,
        row: u16,
        column: u16,
        content: &dyn Display,
    ) -> Result<(), Error>;
    fn clear(&self, screen: &mut dyn Write) -> Result<(), Error>;
    fn get_size(&self) -> Result<TermSize, Error>;
}

pub struct TermionTerminal;

impl TermionTerminal {
    pub fn new() -> Self {
        Self {}
    }
}

impl Terminal for TermionTerminal {
    fn write(
        &self,
        screen: &mut dyn Write,
        row: u16,
        column: u16,
        content: &dyn Display,
    ) -> Result<(), Error> {
        write!(screen, "{}{}", cursor::Goto(column, row), content)
    }

    fn clear(&self, screen: &mut dyn Write) -> Result<(), Error> {
        write!(screen, "{}{}", cursor::Goto(1, 1), clear::All)
    }

    fn get_size(&self) -> Result<TermSize, Error> {
        let terminal_size = match termion::terminal_size() {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        Ok(TermSize {
            height: terminal_size.1,
            width: terminal_size.0,
        })
    }
}
