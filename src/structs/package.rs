use crate::{structs::reason::Reason, version::ChangeType};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Package {
    pub name: String,
    pub required_by: Vec<String>,
    pub required_by_optional: Vec<String>,
    pub dependencies: Vec<String>,
    pub dependencies_optional: Vec<String>,
    pub provides: Option<Vec<String>>,
    pub reason: Reason,
    //info
    pub version: String,
    pub description: String,
    pub validated: bool,

    //installed for installed packages
    pub installed: Option<String>,

    //updates for available updates
    pub new_version: Option<String>,
    pub new_version_size: Option<usize>,
    pub change_type: Option<ChangeType>,

    //full recursive dependency list
    pub dependencies_count: usize,
}
