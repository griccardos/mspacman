use std::{collections::HashMap, fmt::Display};

use ratatui::widgets::TableState;
use tui_textarea::TextArea;

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
    pub installed: String,
    pub description: String,
    pub validated: bool,
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
    pub packages_installed: Vec<Package>,
    pub packages_all: Vec<Package>,
    pub filtered: Vec<Package>,
    pub centre_table_state: TableState,
    pub left_table_state: TableState,
    pub right_table_state: TableState,
    pub provides_table_state: TableState,
    pub focus: Focus,
    pub sort: Sort,
    pub prev: Vec<String>,
    pub only_expl: bool,
    pub only_foreign: bool,
    pub only_orphans: bool,
    pub filter: String,
    pub show_info: bool,
    pub show_help: bool,
    pub show_providing: bool,
    pub hide_columns: HashMap<usize, bool>,
    pub sort_by: (usize, Sort),
    pub message: String,
    pub selected: Vec<String>,
    //for searching
    pub searching: bool,
    pub search_input: TextArea<'static>,
    //for command
    pub command: String,
    pub show_command: bool,
    pub tab: Tab,
    //for showing all/installed
    pub only_installed: bool,
}

#[derive(Debug, Default)]
pub enum Tab {
    #[default]
    Installed,
    Packages,
}
impl Tab {
    pub fn values() -> Vec<String> {
        vec![Tab::Installed.to_string(), Tab::Packages.to_string()]
    }

    pub(crate) fn next(&mut self) {
        *self = match self {
            Tab::Installed => Tab::Packages,
            Tab::Packages => Tab::Installed,
        };
    }
}
impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tab::Installed => write!(f, "Installed"),
            Tab::Packages => write!(f, "Packages"),
        }
    }
}

//for select ratatui::Tabs
impl From<&Tab> for Option<usize> {
    fn from(tab: &Tab) -> Self {
        match tab {
            Tab::Installed => Some(0),
            Tab::Packages => Some(1),
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
    Command(char),
}
