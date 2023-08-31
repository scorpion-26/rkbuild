use std::io::{self, Stdout};

use anyhow::{Context, Result};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::CrosstermBackend, terminal::CompletedFrame, Frame, Terminal};

pub type Backend = CrosstermBackend<Stdout>;

pub struct Tui {
    terminal: Terminal<Backend>,
}

impl Tui {
    pub fn new() -> Result<Self> {
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
            .context("Failed to create terminal")?;

        enable_raw_mode().context("Failed to enable raw mode!")?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)
            .context("Failed to enable alternate screen!")?;
        Ok(Self { terminal })
    }

    pub fn draw<Fun>(&mut self, fun: Fun) -> Result<CompletedFrame, std::io::Error>
    where
        Fun: FnOnce(&mut Frame<Backend>),
    {
        self.terminal.draw(fun)
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        disable_raw_mode()
            .context("Failed to disabled raw mode!")
            .unwrap();
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
            .context("Failed to disable alternate screen!")
            .unwrap();
        self.terminal
            .show_cursor()
            .context("Unable to show cursor!")
            .unwrap();
    }
}
