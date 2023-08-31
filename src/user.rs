use std::{
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{tui};
use crossterm::event::KeyEvent;
use ratatui::{prelude::Rect, Frame};

pub enum ChoiceResult {
    Continue,
    Remove,
}

pub struct Choice<'a> {
    pub render_func: Box<
        dyn FnMut(&mut Frame<tui::Backend>, Rect, &Option<Vec<KeyEvent>>) -> ChoiceResult
            + 'a
            + Send
            + Sync,
    >,
}

pub struct UserInputFuture {
    rec: Receiver<()>,
}

pub struct UserInput<'a> {
    current_choice: Option<Choice<'a>>,
    event_queue: Option<Vec<KeyEvent>>,
    sender: Option<Sender<()>>,
}

impl<'a> Choice<'a> {
    pub fn new<RenFn>(render_func: RenFn) -> Self
    where
        RenFn: FnMut(&mut Frame<tui::Backend>, Rect, &Option<Vec<KeyEvent>>) -> ChoiceResult
            + 'a
            + Send
            + Sync,
    {
        Choice {
            render_func: Box::new(render_func),
        }
    }
}

impl UserInputFuture {
    pub fn wait(&self) {
        self.rec.recv().expect("Failed to receive");
    }
}

impl<'a> UserInput<'a> {
    pub fn new() -> Self {
        UserInput {
            current_choice: None,
            event_queue: None,
            sender: None,
        }
    }

    pub fn set(&mut self, choice: Choice<'a>) -> UserInputFuture {
        self.current_choice = Some(choice);

        // Create the future to wait on
        let (sender, rec) = channel();

        self.sender = Some(sender);
        UserInputFuture { rec }
    }

    pub fn unset(&mut self) {
        self.current_choice = None;
        // Resolve future
        if let Some(sen) = &mut self.sender.take() {
            sen.send(()).expect("Failed to send!");
        }
    }

    pub fn tick(&mut self, event_queue: &Vec<KeyEvent>) {
        self.event_queue = Some(event_queue.clone());
    }

    pub fn draw(&mut self, frame: &mut Frame<tui::Backend>, draw_area: Rect) {
        if let Some(choice) = &mut self.current_choice {
            let choice_result = (choice.render_func)(frame, draw_area, &self.event_queue);
            match choice_result {
                ChoiceResult::Continue => {}
                ChoiceResult::Remove => self.unset(),
            }
        }
    }
}
