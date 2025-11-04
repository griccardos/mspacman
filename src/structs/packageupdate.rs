use crate::version::ChangeType;

#[derive(Debug, Clone, PartialEq)]
pub struct PackageUpdate {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub change_type: ChangeType,
}
