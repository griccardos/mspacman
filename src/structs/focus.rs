use std::fmt::Display;

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Focus {
    #[default]
    InstalledTab,
    PackagesTab,
    UpdatesTab,
    Help,
}

impl Display for Focus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
