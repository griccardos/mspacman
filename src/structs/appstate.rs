use std::time::Duration;

use crate::{
    structs::{package::Package, tab::Tab, timedstring::TimedString},
    widgets::{installed::InstalledWidget, packages::PackagesWidget, update::UpdateWidget},
};

pub struct AppState {
    pub packages: Vec<Package>,
    pub show_info: bool,
    pub show_help: bool,
    pub message: TimedString,

    //for command
    pub command: String,
    pub tab: Tab,
    //tabs
    pub update_widget: UpdateWidget,
    pub packages_widget: PackagesWidget,
    pub installed_widget: InstalledWidget,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            packages: Vec::new(),
            show_info: true,
            show_help: false,
            message: TimedString::new("", Duration::from_secs(5)),
            command: String::new(),
            tab: Tab::Installed,
            update_widget: UpdateWidget::default(),
            packages_widget: PackagesWidget::default(),
            installed_widget: InstalledWidget::default(),
        }
    }
}
