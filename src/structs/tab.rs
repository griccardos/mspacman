use std::fmt::Display;

#[derive(Debug, Default, PartialEq)]
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
