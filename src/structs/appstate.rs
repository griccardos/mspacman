use crate::{
    structs::{package::Package, tab::Tab},
    widgets::{installed::InstalledWidget, packages::PackagesWidget, update::UpdateWidget},
};

#[derive(Debug, Default)]
pub struct AppState {
    pub packages: Vec<Package>,
    pub show_info: bool,
    pub show_help: bool,
    pub message: String,

    //for command
    pub command: String,
    pub tab: Tab,
    //tabs
    pub update_widget: UpdateWidget,
    pub packages_widget: PackagesWidget,
    pub installed_widget: InstalledWidget,
}
