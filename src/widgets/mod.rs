use ratatui::crossterm::event::KeyEvent;

use crate::structs::{event::EventResult, package::Package};

pub mod installed;
pub mod packages;
pub mod table;
pub mod update;

pub trait Commands {
    ///key, description, status description
    fn command_descriptions(&self) -> Vec<(&str, &str, &str)>;
    fn handle_key_event(&mut self, key: &KeyEvent) -> Option<EventResult>;
}

pub trait CurrentPackage {
    fn current_package(&self) -> Option<&Package>;
}
