use ratatui::{
    prelude::{Corner, Rect},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::tui;

pub struct Log<'a> {
    log_items: Vec<ListItem<'a>>,
}

impl<'a> Log<'a> {
    pub fn new() -> Self {
        Log {
            log_items: Vec::new(),
        }
    }

    pub fn append(&mut self, item: String) {
        self.log_items.push(ListItem::new(item));
    }

    pub fn clean(&mut self) {
        self.log_items.clear();
    }

    pub fn replace_newest(&mut self, item: String) {
        self.log_items.pop();
        self.log_items.push(ListItem::new(item));
    }

    pub fn draw(&mut self, frame: &mut Frame<tui::Backend>, draw_area: Rect) {
        // Trim list
        // The border takes two rows, so we have place for draw_area.height - 2 elements
        let num_elements = draw_area.height as usize - 2;
        if num_elements < self.log_items.len() {
            // Split off n elements from the front
            let split_idx = self.log_items.len() - num_elements;
            self.log_items = self.log_items.split_off(split_idx);
        }

        let list_widget = List::new(self.log_items.clone())
            .block(Block::default().borders(Borders::ALL).title("Log"))
            .start_corner(Corner::TopLeft);
        frame.render_widget(list_widget, draw_area);
    }
}
