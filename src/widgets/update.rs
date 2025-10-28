use std::{collections::HashMap, error::Error};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    widgets::{List, Row, StatefulWidget, Table, TableState, Widget},
};

use crate::{
    structs::{EventCommand, EventResult, PackageUpdate},
    version::ChangeType,
};

///Keep state inside the widget. Need to store in your app state
#[derive(Debug, Default, Clone)]
pub struct UpdateWidget {
    data: Vec<PackageUpdate>,
    filtered: Vec<PackageUpdate>,
    pub(crate) table_state: TableState,
}
impl UpdateWidget {
    pub fn new() -> Self {
        Self {
            data: vec![],
            filtered: vec![],
            table_state: TableState::default(),
        }
    }
    pub fn set_data(&mut self, data: &[PackageUpdate]) {
        if data == self.data.as_slice() {
            return;
        }
        self.data = data.iter().cloned().collect();
        self.filtered = self.data.clone();
        if !self.filtered.is_empty() {
            self.table_state.select(Some(0));
        }
    }
    pub fn set_filter(&mut self, filter: Option<ChangeType>) {
        match filter {
            Some(change_type) => {
                self.filtered = self
                    .data
                    .iter()
                    .filter(|pkg| pkg.change_type >= change_type)
                    .cloned()
                    .collect();
            }
            None => {
                self.filtered = self.data.clone();
            }
        }
    }
    pub fn up(&mut self) {
        if self.filtered.is_empty() {
            self.table_state.select(None);
            return;
        }
        if let Some(selected) = self.table_state.selected() {
            let new_selected = if selected == 0 { 0 } else { selected - 1 };
            self.table_state.select(Some(new_selected));
        } else {
            self.table_state.select(Some(0));
        }
    }
    pub fn down(&mut self) {
        if self.filtered.is_empty() {
            self.table_state.select(None);
            return;
        }
        if let Some(selected) = self.table_state.selected() {
            let new_selected = (selected + 1).min(self.filtered.len() - 1);
            self.table_state.select(Some(new_selected));
        } else {
            self.table_state.select(Some(0));
        }
    }

    pub(crate) fn handle_key_event(
        &mut self,
        key: &KeyEvent,
    ) -> Result<EventResult, Box<dyn Error>> {
        match key.code {
            KeyCode::Char('s') => {
                return Ok(EventResult::Queue(vec![
                    //select all visible updates
                    EventResult::Select(self.filtered.iter().map(|u| u.name.clone()).collect()),
                    //sync selected updates
                    EventResult::Command(EventCommand::SyncUpdateSelected),
                ]));
            }
            KeyCode::Char('a') => self.set_filter(None),
            KeyCode::Char('m') => {
                self.set_filter(Some(ChangeType::Major));
            }
            KeyCode::Char('n') => {
                self.set_filter(Some(ChangeType::Minor));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.down();
            }

            _ => {}
        }

        Ok(EventResult::None)
    }
}

impl Widget for UpdateWidget {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let mut map: HashMap<ChangeType, usize> = Default::default();
        for u in &self.data {
            *map.entry(u.change_type.clone()).or_default() += 1;
        }
        let mut counts = map
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect::<Vec<(ChangeType, usize)>>();
        counts.sort_by_key(|k| k.0.clone());
        let mut counts = counts
            .iter()
            .map(|(k, v)| format!("{:?}: {}", k, v))
            .collect::<Vec<String>>();
        counts.insert(0, format!("Total: {}", self.data.len()));
        let commands = vec![
            "s: Update visible".to_string(),
            "a: Show all".to_string(),
            "m: Show major changes and up".to_string(),
            "n: Show minor changes and up".to_string(),
        ];

        let top_bottom = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min((commands.len() + counts.len()) as u16),
                Constraint::Percentage(100),
            ])
            .split(area);

        let all = commands.iter().chain(counts.iter()).map(|s| s.as_str());
        let list = List::new(all).highlight_symbol(">");

        let table = Table::new(
            self.filtered.into_iter().map(|r| {
                Row::new(vec![
                    r.name,
                    r.current_version,
                    r.new_version,
                    format!("{:?}", r.change_type),
                ])
            }),
            [
                Constraint::Percentage(40),
                Constraint::Length(20),
                Constraint::Length(20),
                Constraint::Length(20),
            ],
        )
        .header(
            Row::new(vec!["Name", "Installed", "Latest", "Type"])
                .style(Style::default().underlined().bold()),
        )
        .row_highlight_style(Style::new().bg(Color::Yellow));
        <List as Widget>::render(list, top_bottom[0], buf);
        <Table as StatefulWidget>::render(table, top_bottom[1], buf, &mut self.table_state.clone());
    }
}
