use crossterm::event::KeyCode;
use ratatui::{layout::Constraint, style::Color, widgets::Widget};

use crate::{
    structs::{
        event::{EventCommand, EventResult},
        package::Package,
    },
    widgets::{
        Commands, CurrentPackage,
        table::{TableRow, TableWidget},
    },
};

#[derive(Debug, Clone)]
pub struct PackagesWidget {
    data: Vec<Package>,
    table: TableWidget,
}
impl Default for PackagesWidget {
    fn default() -> Self {
        Self {
            data: vec![],
            table: TableWidget::new(
                &["Name", "Installed", "Info"],
                vec![
                    Constraint::Percentage(30),
                    Constraint::Length(19),
                    Constraint::Percentage(70),
                ],
            ),
        }
    }
}

impl PackagesWidget {
    pub fn set_data(&mut self, data: &[Package]) {
        if data == self.data {
            return;
        }
        self.data = data.to_vec();
        self.table.set_data(
            self.data
                .iter()
                .map(|pkg| {
                    TableRow::new(vec![
                        pkg.name.clone(),
                        pkg.installed.clone().unwrap_or_default(),
                        pkg.description.clone(),
                    ])
                    .with_highlight(if pkg.installed.is_none() {
                        None
                    } else {
                        Some(Color::Green)
                    })
                })
                .collect(),
        );
        self.update_title();
    }
    fn update_title(&mut self) {
        let filtered = self.table.rows().len();
        let installed = self
            .table
            .rows()
            .iter()
            .filter(|r| !r.cells[1].is_empty())
            .count();
        self.table
            .set_title(&format!("{} Packages ({} installed)", filtered, installed));
    }
}
impl Widget for PackagesWidget {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        self.table.render(area, buf);
    }
}
impl Commands for PackagesWidget {
    fn command_descriptions(&self) -> Vec<(&str, &str, &str)> {
        vec![("u", "Update/Install package", "Update/Install")]
    }

    fn handle_key_event(&mut self, key: &crossterm::event::KeyEvent) -> Option<EventResult> {
        if self.table.handle_key_event(key) {
            self.update_title(); //may have filtered
            return Some(EventResult::None);
        }
        if let KeyCode::Char('u') = key.code {
            let packs = self
                .table
                .get_selected()
                .iter()
                .map(|&i| i.cells[0].clone())
                .collect::<Vec<_>>();
            return Some(EventResult::Command(EventCommand::InstallOrUpdateSelected(
                packs,
            )));
        }

        None
    }
}

impl CurrentPackage for PackagesWidget {
    fn current_package(&self) -> Option<&Package> {
        self.table
            .get_current()
            .and_then(|a| self.data.iter().find(|b| b.name == a.cells[0]))
    }
}
