use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Color,
    widgets::Widget,
};

use crate::{
    structs::{event::EventResult, package::Package, reason::Reason},
    widgets::{
        Commands, CurrentPackage,
        table::{TableFocus, TableRow, TableWidget},
    },
};

#[derive(Clone, Debug)]
pub struct InstalledWidget {
    data: Vec<Package>,
    filter_orphans: bool,
    filter_foreign: bool,
    filter_explicit: bool,
    show_providing: bool,

    pub prev: Vec<String>,

    focus: FocusedTable,
    previous_focus: FocusedTable,

    left: TableWidget,
    right: TableWidget,
    centre: TableWidget,
    provides: TableWidget,
}

impl Default for InstalledWidget {
    fn default() -> Self {
        Self {
            data: vec![],
            filter_explicit: false,
            filter_foreign: false,
            filter_orphans: false,
            prev: vec![],
            focus: FocusedTable::Centre,
            previous_focus: FocusedTable::Centre,
            left: TableWidget::new(&["Name"], vec![Constraint::Percentage(100)]).with_no_focus(),
            centre: TableWidget::new(
                &["Name", "Reason", "ReqBy", "Foreign", "Installed"],
                vec![
                    Constraint::Percentage(50),
                    Constraint::Percentage(15),
                    Constraint::Min(5),
                    Constraint::Min(3),
                    Constraint::Length(19),
                ],
            ),
            right: TableWidget::new(&["Name"], vec![Constraint::Percentage(100)]).with_no_focus(),
            provides: TableWidget::new(&[], vec![Constraint::Percentage(100)]).with_no_focus(),
            show_providing: false,
        }
    }
}

impl InstalledWidget {
    pub fn set_data(&mut self, data: Vec<Package>) {
        self.data = data;
        self.filter_data();
    }
    fn filter_data(&mut self) {
        //installed
        let packs: Vec<_> = self
            .data
            .iter()
            .filter(|p| p.installed.is_some())
            .filter(|p| !self.filter_explicit || p.reason == Reason::Explicit) //only show explicit packages
            .filter(|p| !self.filter_foreign || !p.validated) //only show foreign packages
            .filter(|p| {
                !self.filter_orphans
                    || p.required_by.is_empty()
                        && p.optional_for.is_empty()
                        && p.reason == Reason::Dependency
            }) //only show orphans
            .cloned()
            .collect();
        let rows: Vec<TableRow> = packs
            .iter()
            .map(|pack| {
                let highlighted = if pack.reason == Reason::Explicit {
                    Some(Color::Green)
                } else {
                    None
                };
                TableRow {
                    cells: vec![
                        pack.name.clone(),
                        format!("{:?}", pack.reason),
                        pack.required_by.len().to_string(),
                        format!("{}", if pack.validated { "" } else { "X" }),
                        pack.installed.clone().expect("filtered installed only"),
                    ],
                    highlight: highlighted,
                }
            })
            .collect();

        self.centre.set_data(rows);

        let count = self.centre.rows().len();
        let local = self
            .centre
            .rows()
            .iter()
            .filter(|p| p.cells[3].is_empty())
            .count();
        let foreign = count - local;
        let extra = if foreign > 0 {
            format!(" ({local} pacman, {foreign} foreign)")
        } else {
            "".to_string()
        };

        let mut filters = vec![];
        if self.filter_explicit {
            filters.push("Explicit")
        }
        if self.filter_foreign {
            filters.push("Foreign");
        }
        if self.filter_orphans {
            filters.push("Orphans");
        }
        let filters = if filters.is_empty() {
            String::new()
        } else {
            format!("Filters: {}", filters.join(", "))
        };

        let title = format!("Installed {count}{extra} {filters}");
        self.centre.set_title(&title);

        self.update_dependency_tables();
    }

    fn cycle_focus_horiz(&mut self, arg: i32) {
        let Some(pack) = self.current_package() else {
            return;
        };
        let left_count = pack.dependencies.len();
        let right_count = pack.required_by.len();

        self.change_focus(if arg > 0 {
            match self.focus {
                FocusedTable::Left => FocusedTable::Centre,
                FocusedTable::Centre if right_count > 0 => FocusedTable::Right,
                _ => self.focus, //else keep
            }
        } else {
            match self.focus {
                FocusedTable::Centre if left_count > 0 => FocusedTable::Left,
                FocusedTable::Right => FocusedTable::Centre,
                _ => self.focus, //else keep
            }
        });
    }
    fn cycle_focus_vert(&mut self) {
        if self.show_providing {
            self.change_focus(match self.focus {
                FocusedTable::Providing => self.previous_focus,
                _ => FocusedTable::Providing,
            });
        }
    }

    fn update_dependency_tables(&mut self) {
        //dependents
        let Some(pack) = self.current_package() else {
            return;
        };
        let pack = pack.clone();
        let count = pack.required_by.len();
        let rows: Vec<TableRow> = pack
            .required_by
            .iter()
            .map(|dep| TableRow {
                cells: vec![dep.clone()],
                highlight: None,
            })
            .collect();
        self.right.set_data(rows);
        self.right.set_title(&format!("Required by {count}"));

        //dependencies
        let rows: Vec<TableRow> = pack
            .dependencies
            .iter()
            .map(|dep| {
                let col: Option<Color> = match self.get_pack(dep) {
                    Some(p) if p.reason == Reason::Explicit => Some(Color::Green),
                    Some(_) => None,
                    None => Some(Color::Red),
                };

                TableRow {
                    cells: vec![dep.clone()],
                    highlight: col,
                }
            })
            .collect();
        let count = pack.dependencies.len();

        let title = format!("Depends on {count}");
        self.left.set_title(&title);
        self.left.set_data(rows);

        //provides
        let rows: Vec<TableRow> = pack
            .provides
            .iter()
            .filter(|p| !p.ends_with('/'))
            .map(|p| TableRow {
                cells: vec![p.clone()],
                highlight: None,
            })
            .collect();
        self.provides.set_title(&format!("{} files", rows.len()));
        self.provides.set_data(rows);
    }
    fn get_pack(&self, name: &str) -> Option<&Package> {
        self.data.iter().find(|p| p.name == name)
    }
    fn goto_package(&mut self, name: &str) {
        self.change_focus(FocusedTable::Centre);
        //find new index in table rows
        let new_index = self
            .centre
            .rows()
            .iter()
            .position(|p| p.cells[0] == name)
            .unwrap_or_default();

        self.centre.set_current(Some(new_index));
    }
    fn change_focus(&mut self, new_focus: FocusedTable) {
        if new_focus == self.focus {
            return;
        }

        self.previous_focus = self.focus;
        self.focus = new_focus;

        self.update_focus();
    }

    fn update_focus(&mut self) {
        self.left.focus(TableFocus::Unfocused);
        self.centre.focus(TableFocus::UnfocusedDimmed);
        self.right.focus(TableFocus::Unfocused);
        self.provides.focus(TableFocus::Unfocused);
        match self.focus {
            FocusedTable::Left => self.left.focus(TableFocus::Focused),
            FocusedTable::Centre => self.centre.focus(TableFocus::Focused),
            FocusedTable::Right => self.right.focus(TableFocus::Focused),
            FocusedTable::Providing => self.provides.focus(TableFocus::Focused),
        }
    }
    fn handle_enter(&mut self) {
        let new = match self.focus {
            FocusedTable::Left => self.left.get_current(),
            FocusedTable::Right => self.right.get_current(),
            _ => return,
        };
        let Some(new) = new else {
            return;
        };
        let new_name = new.cells[0].clone();
        self.prev.push(
            self.current_package()
                .map(|p| p.name.clone())
                .unwrap_or_default(),
        );
        self.goto_package(&new_name);
    }
}

impl Commands for InstalledWidget {
    fn command_descriptions(&self) -> Vec<(&str, &str, &str)> {
        vec![
            ("r", "Remove selected packages", "Remove"),
            ("e", "Explicitly installed packages", "Explicit"),
            ("f", "Foreign packages", "Foreign"),
            ("o", "Orphaned packages", "Orphans"),
            ("p", "View files provided by package", "Provides"),
            ("P", "Focus providing packages table", ""),
            ("←/h", "Left dependency table", ""),
            ("→/l", "Right dependent table", ""),
            ("Backspace", "Go to previous package", ""),
        ]
    }

    fn handle_key_event(&mut self, _key: &crossterm::event::KeyEvent) -> Option<EventResult> {
        let handled = match self.focus {
            FocusedTable::Left => self.left.handle_key_event(_key),
            FocusedTable::Centre => self.centre.handle_key_event(_key),
            FocusedTable::Right => self.right.handle_key_event(_key),
            FocusedTable::Providing => self.provides.handle_key_event(_key),
        };
        if handled {
            return Some(EventResult::None);
        }

        match _key.code {
            KeyCode::Char('r') => {
                let selected_names = self
                    .centre
                    .get_selected()
                    .into_iter()
                    .filter_map(|c| c.cells.get(0))
                    .cloned()
                    .collect();

                return Some(EventResult::Command(
                    crate::structs::event::EventCommand::RemoveSelected(selected_names),
                ));
            }
            KeyCode::Char('e') => self.filter_explicit = !self.filter_explicit,
            KeyCode::Char('f') => self.filter_foreign = !self.filter_foreign,
            KeyCode::Char('o') => self.filter_orphans = !self.filter_orphans,
            KeyCode::Left | KeyCode::Char('h') => self.cycle_focus_horiz(-1),
            KeyCode::Right | KeyCode::Char('l') => self.cycle_focus_horiz(1),
            KeyCode::Char('P') => self.cycle_focus_vert(),
            KeyCode::Char('p') => self.show_providing = !self.show_providing,
            KeyCode::Backspace => {
                if let Some(prev) = self.prev.pop() {
                    self.goto_package(&prev);
                }
            }
            KeyCode::Enter => self.handle_enter(),
            _ => {}
        }
        self.filter_data();
        None
    }
}

impl Widget for InstalledWidget {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let pr = if self.show_providing {
            ((area.height as f32 * 0.5) as u16).max(5)
        } else {
            0
        };
        let verti = Layout::vertical(vec![
            Constraint::Percentage(100 - pr),
            Constraint::Length(pr),
        ])
        .split(area);
        let areas = Layout::horizontal(vec![
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(verti[0]);

        self.left.render(areas[0], buf);
        self.centre.render(areas[1], buf);
        self.right.render(areas[2], buf);

        if self.show_providing {
            self.provides.render(verti[1], buf);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FocusedTable {
    Left,
    Centre,
    Right,
    Providing,
}

impl CurrentPackage for InstalledWidget {
    fn current_package(&self) -> Option<&Package> {
        let Some(curr) = self.centre.get_current() else {
            return None;
        };
        self.data.iter().find(|p| p.name == curr.cells[0])
    }
}
