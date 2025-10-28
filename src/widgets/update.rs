use std::{collections::HashMap, error::Error};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{List, Widget},
};

use crate::{
    structs::{EventCommand, EventResult, PackageUpdate},
    version::ChangeType,
    widgets::table::TableWidget,
};

///Keep state inside the widget. Need to store in your app state
#[derive(Debug, Default, Clone)]
pub struct UpdateWidget {
    data: Vec<PackageUpdate>,
    filtered: Vec<PackageUpdate>,
    table: TableWidget,
}
impl UpdateWidget {
    pub fn new() -> Self {
        Self {
            data: vec![],
            filtered: vec![],
            table: TableWidget::new(
                vec![
                    "Name".to_string(),
                    "Installed".to_string(),
                    "Latest".to_string(),
                    "Type".to_string(),
                ],
                vec![
                    Constraint::Percentage(40),
                    Constraint::Length(20),
                    Constraint::Length(20),
                    Constraint::Length(20),
                ],
            ),
        }
    }
    pub fn set_data(&mut self, data: &[PackageUpdate]) {
        if data == self.data.as_slice() {
            return;
        }
        self.data = data.iter().cloned().collect();
        self.filtered = self.data.clone();
        self.set_table_data();
    }
    pub fn set_table_data(&mut self) {
        self.table.set_data(
            self.filtered
                .iter()
                .map(|r| {
                    vec![
                        r.name.clone(),
                        r.current_version.clone(),
                        r.new_version.clone(),
                        format!("{:?}", r.change_type),
                    ]
                })
                .collect(),
        );
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
        self.set_table_data();
    }

    pub(crate) fn handle_key_event(
        &mut self,
        key: &KeyEvent,
    ) -> Result<EventResult, Box<dyn Error>> {
        match key.code {
            KeyCode::Char('s') => return Ok(EventResult::Command(EventCommand::UpdateDatabase)),
            KeyCode::Char('u') => {
                let selected = self.table.get_selected_indices();
                let selected_names = selected
                    .iter()
                    .filter_map(|&i| self.filtered.get(i))
                    .map(|u| u.name.clone())
                    .collect::<Vec<String>>();
                return Ok(EventResult::Queue(vec![
                    //select all visible updates
                    EventResult::Select(selected_names),
                    //sync selected updates
                    EventResult::Command(EventCommand::SyncUpdateSelected),
                ]));
            }
            KeyCode::Char('a') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.table.select_all();
                } else {
                    self.set_filter(None);
                }
            }

            KeyCode::Char('m') => {
                self.set_filter(Some(ChangeType::Major));
            }
            KeyCode::Char('n') => {
                self.set_filter(Some(ChangeType::Minor));
            }
            KeyCode::Esc => self.table.clear_selection(),

            _ => {}
        }
        self.table.handle_key_event(key)?;

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
        counts.reverse();
        let counts = counts
            .iter()
            .map(|(k, v)| format!("{} {}", v, k))
            .collect::<Vec<String>>();
        let selected = self.table.get_selected_indices().len();

        let message = format!(
            "{} Updates ({}) {}",
            self.data.len(),
            counts
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(", "),
            if selected > 0 {
                format!("{} selected", selected)
            } else {
                "".to_string()
            }
        );
        let mut commands = vec![
            "s: Update database".to_string(),
            "u: Update selected packages".to_string(),
            "m: Show major changes and up".to_string(),
            "n: Show minor changes and up".to_string(),
            "a: Show all".to_string(),
            "Ctrl+a: Select All".to_string(),
            "ESC: Clear Selection".to_string(),
        ];
        commands.push(message);

        let top_bottom = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min((commands.len()) as u16),
                Constraint::Percentage(100),
            ])
            .split(area);

        let all = commands.iter().chain(counts.iter()).map(|s| s.as_str());
        let list = List::new(all);

        <List as Widget>::render(list, top_bottom[0], buf);
        self.table.render(top_bottom[1], buf);
    }
}
