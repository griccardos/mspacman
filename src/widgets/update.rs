use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Constraint, style::Color, widgets::Widget};

use crate::{
    structs::{EventCommand, EventResult, PackageUpdate},
    version::ChangeType,
    widgets::{
        Commands,
        table::{TableRow, TableWidget},
    },
};

///Keep state inside the widget. Need to store in your app state
#[derive(Debug, Default, Clone)]
pub struct UpdateWidget {
    data: Vec<PackageUpdate>,
    filtered: Vec<PackageUpdate>,
    table: TableWidget,
    filter: Option<ChangeType>,
}
impl UpdateWidget {
    pub fn new() -> Self {
        Self {
            data: vec![],
            filtered: vec![],
            filter: None,
            table: TableWidget::new(
                &["Name", "Installed", "Latest", "Type"],
                vec![
                    Constraint::Percentage(100),
                    Constraint::Length(25),
                    Constraint::Length(25),
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
        self.filter_data();
    }
    pub fn filter_data(&mut self) {
        match &self.filter {
            Some(change_type) => {
                self.filtered = self
                    .data
                    .iter()
                    .filter(|pkg| &pkg.change_type >= change_type)
                    .cloned()
                    .collect();
            }
            None => {
                self.filtered = self.data.clone();
            }
        }

        self.table.set_data(
            self.filtered
                .iter()
                .map(|r| TableRow {
                    cells: vec![
                        r.name.clone(),
                        r.current_version.clone(),
                        r.new_version.clone(),
                        format!("{:?}", r.change_type),
                    ],
                    highlight: if r.change_type >= ChangeType::Major {
                        Some(Color::Green)
                    } else {
                        None
                    },
                })
                .collect(),
        );
    }
}

impl Widget for UpdateWidget {
    fn render(mut self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
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
        let filters = if let Some(f) = self.filter {
            format!("Filters: >={:?}", f)
        } else {
            String::new()
        };

        let message = format!(
            "{} Updates ({}) {}",
            self.data.len(),
            counts
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(", "),
            format!("{filters}")
        );
        self.table.set_title(&message);

        self.table.render(area, buf);
    }
}

impl Commands for UpdateWidget {
    fn command_descriptions(&self) -> Vec<(&str, &str)> {
        vec![
            ("s", "Sync database"),
            ("u", "Update selected packages"),
            ("m", "Show major changes and up"),
            ("n", "Show minor changes and up"),
            ("a", "Show all changes"),
            ("/", "Search table"),
            ("Esc", "Clear selection and filters"),
            ("Ctrl+a", "Toggle select all"),
        ]
    }
    fn handle_key_event(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let handled = self.table.handle_key_event(key);
        if handled {
            return Some(EventResult::None); //handled so do nothing more
        };

        match key.code {
            KeyCode::Char('s') => return Some(EventResult::Command(EventCommand::UpdateDatabase)),
            KeyCode::Char('u') => {
                let selected_names = self
                    .table
                    .get_selected()
                    .into_iter()
                    .filter_map(|c| c.cells.get(0))
                    .cloned()
                    .collect();

                return Some(EventResult::Queue(vec![
                    //select all visible updates
                    EventResult::Select(selected_names),
                    //sync selected updates
                    EventResult::Command(EventCommand::SyncUpdateSelected),
                ]));
            }
            KeyCode::Char('a') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if self.table.get_selected().len() == self.filtered.len() {
                        self.table.clear_selection();
                    } else {
                        self.table.select_all();
                    }
                } else {
                    self.filter = None;
                    self.filter_data();
                }
            }

            KeyCode::Char('m') => {
                self.filter = Some(ChangeType::Major);
                self.filter_data();
            }
            KeyCode::Char('n') => {
                self.filter = Some(ChangeType::Minor);
                self.filter_data();
            }
            KeyCode::Esc => {
                self.table.clear_selection();
                self.filter = None;
                self.filter_data();
            }

            _ => {}
        }

        None
    }
}
