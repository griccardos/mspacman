use crossterm::event::KeyEvent;

use crate::structs::EventResult;

pub mod table;
pub mod update;

pub trait Commands {
    fn command_descriptions(&self) -> Vec<(&str, &str)>;
    fn handle_key_event(&mut self, key: &KeyEvent) -> Option<EventResult>;
}
