use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    tui,
    user::{Choice, ChoiceResult},
};

pub enum TextInputType {
    String,
    Version,
}
pub struct TextInput<'a> {
    choice: Option<Choice<'a>>,
    str: Arc<Mutex<String>>,
}

pub struct EnumInput<'a> {
    choice: Option<Choice<'a>>,
    chosen_idx: Arc<Mutex<usize>>,
}

impl<'a> TextInput<'a> {
    pub fn new(input_type: TextInputType, title: &'static str) -> Self {
        let string = Arc::new(Mutex::new(String::new()));
        let str_mem = string.clone();
        let mut cursor_posx: u16 = 0;
        let render_func =
            move |frame: &mut Frame<tui::Backend>, area: Rect, queue: &Option<Vec<KeyEvent>>| {
                if let Some(queue) = queue {
                    for key in queue {
                        match key.code {
                            KeyCode::Enter => return ChoiceResult::Remove,
                            KeyCode::Backspace => {
                                let mut string = string.lock().unwrap();
                                if string.len() > 0 {
                                    string.pop();
                                    cursor_posx -= 1;
                                }
                            }
                            KeyCode::Char(c) => {
                                if c.is_numeric()
                                    || c == '.'
                                    || matches!(input_type, TextInputType::String)
                                {
                                    string.lock().unwrap().push(c);
                                    cursor_posx += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                let text = Paragraph::new(string.lock().unwrap().clone()).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(Style::default().fg(Color::LightGreen)),
                );
                frame.render_widget(text, area);

                frame.set_cursor(area.x + cursor_posx + 1, area.y + 1);
                ChoiceResult::Continue
            };

        TextInput {
            choice: Some(Choice::new(render_func)),
            str: str_mem,
        }
    }

    pub fn choice(&mut self) -> Choice<'a> {
        self.choice.take().expect("Choice can only be called once!")
    }

    pub fn output(&self) -> Arc<Mutex<String>> {
        self.str.clone()
    }
}

impl<'a> EnumInput<'a> {
    pub fn new(choices: Vec<String>, title: &'static str) -> Self {
        let chosen_idx = Arc::new(Mutex::new(0));
        let chosen_idx_mem = chosen_idx.clone();
        let mut state: ListState = ListState::default();
        state.select(Some(0));
        let render_func =
            move |frame: &mut Frame<tui::Backend>, area: Rect, queue: &Option<Vec<KeyEvent>>| {
                let mut selection_idx = state.selected().unwrap();
                if let Some(queue) = queue {
                    for key in queue {
                        match key.code {
                            KeyCode::Enter => {
                                *chosen_idx.lock().unwrap() = selection_idx;
                                return ChoiceResult::Remove;
                            }
                            KeyCode::Up => {
                                if selection_idx > 0 {
                                    selection_idx -= 1;
                                }
                            }
                            KeyCode::Down => {
                                selection_idx += 1;
                            }
                            _ => {}
                        }
                    }
                }
                selection_idx = selection_idx.clamp(0, choices.len() - 1);
                state.select(Some(selection_idx));

                let choices: Vec<ListItem> = choices
                    .iter()
                    .map(|i| ListItem::new(vec![Line::from(i.clone())]))
                    .collect();
                let list = List::new(choices)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(title)
                            .border_style(Style::default().fg(Color::LightGreen)),
                    )
                    .highlight_style(Style::default().fg(Color::Yellow))
                    .highlight_symbol("> ");
                frame.render_stateful_widget(list, area, &mut state);

                ChoiceResult::Continue
            };

        EnumInput {
            choice: Some(Choice::new(render_func)),
            chosen_idx: chosen_idx_mem,
        }
    }

    pub fn choice(&mut self) -> Choice<'a> {
        self.choice.take().expect("Choice can only be called once!")
    }

    pub fn output(&self) -> Arc<Mutex<usize>> {
        self.chosen_idx.clone()
    }
}
