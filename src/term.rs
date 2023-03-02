use std::io::{stdin, stdout, Error, Write};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

use crossterm::event::{read, Event, KeyCode};
use crossterm::style::Stylize;
use crossterm::style::{self};
use crossterm::terminal::{size, Clear, ClearType};
use crossterm::{QueueableCommand, ExecutableCommand};
use crossterm::{cursor::MoveTo, Result};

pub struct TermSize {
    pub width: u16,
    pub height: u16,
}

pub trait Terminal {
    fn write(&self, screen: &mut dyn Write, row: u16, column: u16, content: &str);
    fn clear(&self, screen: &mut dyn Write);
    fn get_size(&self) -> Result<TermSize>;
    fn on_input(&self) -> Result<Receiver<char>>;
    fn on_resize(&self) -> Result<Receiver<TermSize>>;
}

pub struct TermionTerminal;

impl TermionTerminal {
    pub fn new() -> Self {
        Self {}
    }
}

impl Terminal for TermionTerminal {
    fn write(&self, stdout: &mut dyn Write, row: u16, column: u16, content: &str) {
        stdout.queue(MoveTo(column, row)).unwrap();
        stdout
            .queue(style::PrintStyledContent(content.red()))
            .unwrap();
    }

    fn clear(&self, stdout: &mut dyn Write) {
        stdout.queue(MoveTo(1, 1)).unwrap();
        stdout.execute(Clear(ClearType::All)).unwrap();
    }

    fn get_size(&self) -> Result<TermSize> {
        let size = size().unwrap();
        Ok(TermSize {
            width: size.0,
            height: size.1,
        })
    }

    fn on_input(&self) -> Result<Receiver<char>> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || loop {
            match read().unwrap() {
                Event::Key(event) => match event.code {
                    KeyCode::Char(char) => tx.send(char).unwrap(),
                    _ => {}
                },
                _ => {}
            }
        });

        Ok(rx)
    }

    fn on_resize(&self) -> Result<Receiver<TermSize>> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || loop {
            match read().unwrap() {
                Event::Resize(width, height) => tx.send(TermSize { width, height }).unwrap(),
                _ => {}
            }
        });

        Ok(rx)
    }
}
