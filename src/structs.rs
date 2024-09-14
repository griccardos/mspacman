use std::collections::HashMap;

use ratatui::widgets::TableState;
use tui_textarea::TextArea;

#[derive(Debug, Default, Clone)]
pub struct Package {
    pub name: String,
    pub required_by: Vec<String>,
    pub optional_for: Vec<String>,
    pub dependencies: Vec<String>,
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
    pub packs: Vec<Package>,
    pub filtered: Vec<Package>,
    pub centre_table_state: TableState,
    pub left_table_state: TableState,
    pub right_table_state: TableState,
    pub focus: Focus,
    pub sort: Sort,
    pub prev: Vec<String>,
    pub only_expl: bool,
    pub only_foreign: bool,
    pub only_orphans: bool,
    pub filter: String,
    pub show_info: bool,
    pub show_help: bool,
    pub hide_columns: HashMap<usize, bool>,
    pub sort_by: (usize, Sort),
    pub message: String,
    pub selected: Vec<String>,
    //for searching
    pub searching: bool,
}
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Focus {
    Left,
    #[default]
    Centre,
    Right,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Sort {
    #[default]
    Asc,
    Desc,
}
