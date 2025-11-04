use crate::{structs::reason::Reason, version::ChangeType};

#[derive(Debug, Default, Clone, PartialEq)]
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
