use core::time::Duration;
use libc::{SIGINT, SIGWINCH};
use signal_hook::iterator::Signals;
use std::fmt::Display;
use std::io::{stdin, stdout, Error, Write};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, cursor, terminal_size};

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
    fn on_resize(&self) -> Result<Receiver<TermSize>, Error>;
    fn on_input(&self) -> Result<Receiver<char>, Error>;
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

    fn on_input(&self) -> Result<Receiver<char>, Error> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let stdin = stdin();
            for c in stdin.keys() {
                match c.unwrap() {
                    Key::Char(character) => tx
                        .send(character)
                        .expect("Failed to send pressed key via input channel"),
                    _ => continue,
                };
            }
        });

        Ok(rx)
    }

    fn on_resize(&self) -> Result<Receiver<TermSize>, Error> {
        let mut signals = Signals::new(&[SIGWINCH]).unwrap();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for _ in signals.forever() {
                let terminal_size = termion::terminal_size().unwrap();
                tx.send(TermSize {
                    height: terminal_size.1,
                    width: terminal_size.0,
                })
                .unwrap();
            }
        });

        Ok(rx)
    }
}
