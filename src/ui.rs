use std::{
    time::Duration,
};

use crate::{
    log::Log,
    tui::{self},
    user::UserInput,
};
use anyhow::{Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    prelude::{Constraint, Direction, Layout},
    Frame,
};

pub enum TickResult {
    Ok,
    Exit,
}

pub struct UI<'a> {
    log: Log<'a>,
    input: UserInput<'a>,
}

pub struct UIController<'a> {
    pub ui: UI<'a>,
}

impl<'a> UI<'a> {
    pub fn new() -> Self {
        Self {
            input: UserInput::new(),
            log: Log::new(),
        }
    }

    pub fn log<'b>(&'b mut self) -> &mut Log<'a> {
        &mut self.log
    }

    pub fn input(&mut self) -> &mut UserInput<'a> {
        &mut self.input
    }

    pub fn tick(&mut self) -> Result<TickResult> {
        let mut event_queue: Vec<KeyEvent> = vec![];

        loop {
            match event::poll(Duration::from_millis(0))? {
                true => match event::read()? {
                    Event::Key(key) => {
                        if key.code == KeyCode::Esc {
                            return Ok(TickResult::Exit);
                        } else {
                            event_queue.push(key);
                            continue;
                        }
                    }
                    _ => continue,
                },
                false => break,
            };
        }
        self.input.tick(&event_queue);

        Ok(TickResult::Ok)
    }

    pub fn render(&mut self, frame: &mut Frame<tui::Backend>) {
        let fourth = frame.size().width / 4;
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(fourth),
                Constraint::Length(frame.size().width - fourth),
            ])
            .split(frame.size());

        self.input.draw(frame, chunks[0]);
        self.log.draw(frame, chunks[1]);
    }
}
