use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Constraint, style::Color, widgets::Widget};

use crate::{
    structs::{
        event::{EventCommand, EventResult},
        package::Package,
    },
    utils::thousands,
    version::ChangeType,
    widgets::{
        Commands, CurrentPackage,
        table::{TableRow, TableWidget},
    },
};

///Keep state inside the widget. Need to store in your app state
#[derive(Debug, Clone)]
pub struct UpdateWidget {
    data: Vec<Package>,
    filtered: Vec<Package>,
    table: TableWidget,
    filter: Option<ChangeType>,
}
impl Default for UpdateWidget {
    fn default() -> Self {
        Self {
            data: vec![],
            filtered: vec![],
            filter: None,
            table: TableWidget::new(
                &["Name", "Installed", "Latest", "Type", "        Size"],
                vec![
                    Constraint::Percentage(50),
                    Constraint::Length(25),
                    Constraint::Length(25),
                    Constraint::Length(10),
                    Constraint::Length(15),
                ],
            ),
        }
    }
}
impl UpdateWidget {
    pub fn set_data(&mut self, data: &[Package]) {
        //check if data is the same as current data
        if data == self.data {
            return;
        }
        self.data = data.to_vec();
        self.filtered = self.data.clone();
        self.filter_data();
    }
    pub fn filter_data(&mut self) {
        self.filtered = self
            .data
            .iter()
            .filter(|pkg| pkg.change_type >= self.filter)
            .cloned()
            .collect();

        self.table.set_data(
            self.filtered
                .iter()
                .map(|r| {
                    TableRow::new(vec![
                        r.name.clone(),
                        r.version.clone(),
                        r.new_version.clone().expect("all updates have new_version"),
                        format!("{}", r.change_type.as_ref().unwrap_or(&ChangeType::Major)),
                        format!(
                            "{: >15}",
                            r.new_version_size.map(thousands).unwrap_or_default()
                        ),
                    ])
                    .with_highlight(
                        if r.change_type >= Some(ChangeType::Major) {
                            Some(Color::Green)
                        } else {
                            None
                        },
                    )
                })
                .collect(),
        );
        let mut map: HashMap<ChangeType, usize> = Default::default();
        for u in &self.data {
            *map.entry(u.change_type.clone().expect("All updates have change type"))
                .or_default() += 1;
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
        let filters = if let Some(f) = &self.filter {
            format!("Filters: >={:?}", f)
        } else {
            String::new()
        };

        let message = format!(
            "{} Updates ({}) {filters}",
            self.data.len(),
            counts
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(", "),
        );
        self.table.set_title(&message);
    }
}

impl Widget for UpdateWidget {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        self.table.render(area, buf);
    }
}

impl Commands for UpdateWidget {
    fn command_descriptions(&self) -> Vec<(&str, &str, &str)> {
        vec![
            ("u", "Update selected packages", "Update"),
            ("U", "Update all packages", "Update All"),
            ("m", "Show major changes and up", "Major"),
            ("n", "Show minor changes and up", "Minor"),
            ("a", "Show all changes", "All"),
            ("Enter", "View dependencies", "Dependencies"),
        ]
    }
    fn handle_key_event(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let handled = self.table.handle_key_event(key);
        if handled {
            return Some(EventResult::None); //handled so do nothing more
        };

        match key.code {
            KeyCode::Char('u') => {
                let selected_names = self
                    .table
                    .get_selected()
                    .into_iter()
                    .filter_map(|c| c.cells.first())
                    .cloned()
                    .collect();

                return Some(EventResult::Command(EventCommand::InstallOrUpdateSelected(
                    selected_names,
                )));
            }
            KeyCode::Char('U') => {
                return Some(EventResult::Command(EventCommand::SyncAndUpdateAll));
            }
            KeyCode::Char('a') => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
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
                self.filter = None;
                self.filter_data();
            }
            KeyCode::Enter => {
                //goto installed
                if let Some(pkg) = self.current_package() {
                    return Some(EventResult::GotoInstalled(pkg.name.clone()));
                }
            }

            _ => {}
        }

        None
    }
}

impl CurrentPackage for UpdateWidget {
    fn current_package(&self) -> Option<&Package> {
        self.table
            .get_current()
            .and_then(|a| self.filtered.iter().find(|p| p.name == a.cells[0]))
    }
}
