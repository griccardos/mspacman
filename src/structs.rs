use std::fmt::Display;

use ratatui::layout::Constraint;

use crate::{
    version::ChangeType,
    widgets::{
        table::{TableFocus, TableWidget},
        update::UpdateWidget,
    },
};

#[derive(Debug, Default, Clone)]
pub struct Package {
    pub name: String,
    pub required_by: Vec<String>,
    pub optional_for: Vec<String>,
    pub dependencies: Vec<String>,
    pub provides: Vec<String>,
    pub reason: Reason,
    //info
    pub version: String,
    pub description: String,
    pub validated: bool,

    //installed for installed packages
    pub installed: Option<String>,

    //updates for available updates
    pub new_version: Option<String>,
    pub change_type: Option<ChangeType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageUpdate {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub change_type: ChangeType,
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd)]
pub enum Reason {
    #[default]
    Dependency,
    Explicit,
    Other(String),
}

#[derive(Debug, Default)]
pub struct AppState {
    pub packages: Vec<Package>,
    pub left_table: TableWidget,
    pub right_table: TableWidget,
    pub provides_table: TableWidget,
    focus: Focus,
    focus_previous: Focus,
    pub prev: Vec<String>,
    pub only_expl: bool,
    pub only_foreign: bool,
    pub only_orphans: bool,
    pub show_info: bool,
    pub show_providing: bool,
    pub message: String,
    pub selected: Vec<String>,

    //for command
    pub command: String,
    pub tab: Tab,
    //for showing all/installed
    pub only_installed: bool,
    //tabs
    pub update_widget: UpdateWidget,
    pub packages_table: TableWidget,
    pub installed_table: TableWidget,
}

impl AppState {
    pub(crate) fn new() -> Self {
        AppState {
            packages: vec![],
            show_info: true,
            only_installed: true,
            left_table: TableWidget::new(&["Name"], vec![Constraint::Percentage(100)]),
            right_table: TableWidget::new(&["Name"], vec![Constraint::Percentage(100)]),
            update_widget: UpdateWidget::new(),
            installed_table: TableWidget::new(
                &["Name", "Reason", "ReqBy", "Foreign", "Installed"],
                vec![
                    Constraint::Percentage(50),
                    Constraint::Percentage(15),
                    Constraint::Min(5),
                    Constraint::Min(3),
                    Constraint::Length(19),
                ],
            ),
            packages_table: TableWidget::new(
                &["Name", "Installed"],
                vec![Constraint::Percentage(70), Constraint::Length(19)],
            ),
            ..Default::default()
        }
    }

    pub(crate) fn change_focus(&mut self, new_focus: Focus) {
        self.focus_previous = self.focus;
        self.focus = new_focus;

        self.installed_table.focus(TableFocus::UnfocusedDimmed);
        self.packages_table.focus(TableFocus::UnfocusedDimmed);
        self.left_table.focus(TableFocus::Unfocused);
        self.right_table.focus(TableFocus::Unfocused);
        self.provides_table.focus(TableFocus::Unfocused);
        match self.focus {
            Focus::Left => {
                self.left_table.focus(TableFocus::Focused);
            }
            Focus::Centre => {
                if self.only_installed {
                    self.installed_table.focus(TableFocus::Focused);
                } else {
                    self.packages_table.focus(TableFocus::Focused);
                }
            }
            Focus::Right => {
                self.right_table.focus(TableFocus::Focused);
            }
            Focus::Provides => {
                self.provides_table.focus(TableFocus::Focused);
            }
            _ => {}
        }
    }
    pub(crate) fn restore_focus(&mut self) {
        self.focus = self.focus_previous;
    }
    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub(crate) fn focus_previous(&self) -> Focus {
        self.focus_previous
    }
}

#[derive(Debug, Default)]
pub enum Tab {
    #[default]
    Installed,
    Packages,
    Updates,
}
impl Tab {
    pub fn values() -> Vec<String> {
        vec![
            Tab::Installed.to_string(),
            Tab::Packages.to_string(),
            Tab::Updates.to_string(),
        ]
    }

    pub(crate) fn cycle_next(&mut self) {
        *self = match self {
            Tab::Installed => Tab::Packages,
            Tab::Packages => Tab::Updates,
            Tab::Updates => Tab::Installed,
        };
    }
    pub fn cycle_prev(&mut self) {
        *self = match self {
            Tab::Installed => Tab::Updates,
            Tab::Packages => Tab::Installed,
            Tab::Updates => Tab::Packages,
        };
    }
}

impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tab::Installed => write!(f, "Installed"),
            Tab::Packages => write!(f, "Packages"),
            Tab::Updates => write!(f, "Updates"),
        }
    }
}

//for select ratatui::Tabs
impl From<&Tab> for Option<usize> {
    fn from(tab: &Tab) -> Self {
        match tab {
            Tab::Installed => Some(0),
            Tab::Packages => Some(1),
            Tab::Updates => Some(2),
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Focus {
    Left,
    #[default]
    Centre,
    Right,
    Provides,
    Updates,
    Command,
    Help,
}

impl Display for Focus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Sort {
    #[default]
    Asc,
    Desc,
}

pub enum EventResult {
    None,
    Quit,
    Select(Vec<String>),
    Command(EventCommand),
    Queue(Vec<EventResult>),
}

pub enum EventCommand {
    RemoveSelected,
    UpdateSelected,
    QuerySelected,
    SyncDatabase,
}
