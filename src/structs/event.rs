#[derive(PartialEq)]
pub enum EventResult {
    None,
    Quit,
    Command(EventCommand),
    NeedsUpdate,
    GotoInstalled(String),
}

#[derive(PartialEq)]
pub enum EventCommand {
    RemoveSelected(Vec<String>),
    InstallOrUpdateSelected(Vec<String>),
    QuerySelected(Vec<String>),
    SyncDatabase,
}
