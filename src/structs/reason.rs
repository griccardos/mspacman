#[derive(Debug, Clone, Default, PartialEq, PartialOrd)]
pub enum Reason {
    #[default]
    Dependency,
    Explicit,
    Other(String),
}
