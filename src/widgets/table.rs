use crate::utils::natural_cmp;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Constraint,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Cell, Clear, Row, StatefulWidget, Table, TableState, Widget},
};
use tui_textarea::TextArea;

#[derive(Default, Debug, Clone)]
pub struct TableWidget {
    columns: Vec<String>,
    widths: Vec<Constraint>,
    data: Vec<TableRow>,
    filtered: Vec<TableRow>,
    table_state: TableState,
    sort_by: (usize, Sort),
    selected: Vec<usize>,
    title: Option<String>,
    focus_type: TableFocus,
    search_text_area: TextArea<'static>,
    searching: bool,
}
#[derive(Default, Debug, Clone)]
pub enum TableFocus {
    #[default]
    Focused, //shows selection normally
    UnfocusedDimmed, //hides selection all together
    Unfocused,       //shows selection dimmed
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Sort {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableRow {
    pub cells: Vec<String>,
    pub highlight: Option<Color>,
}

impl TableWidget {
    pub fn new(columns: &[&str], widths: Vec<Constraint>) -> Self {
        Self {
            columns: columns.iter().map(|s| s.to_string()).collect(),
            widths,
            data: vec![],
            filtered: vec![],
            table_state: TableState::default(),
            sort_by: (0, Sort::Asc),
            selected: vec![],
            title: None,
            focus_type: TableFocus::Focused,
            search_text_area: get_textarea(),
            searching: false,
        }
    }
    pub fn with_no_focus(self) -> Self {
        Self {
            focus_type: TableFocus::Unfocused,
            ..self
        }
    }

    ///return true if event was handled and should not be processed further
    pub(crate) fn handle_key_event(&mut self, key: &KeyEvent) -> bool {
        if self.searching {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.searching = false;
                    return true;
                }
                _ => {}
            }
            self.search_text_area.input(*key);
            self.update_filtered();

            return true; //dont process other items
        }
        // self.columns[0] = format!("handling {}", key.code);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.safe_move(-1),
            KeyCode::Down | KeyCode::Char('j') => self.safe_move(1),
            KeyCode::Esc => {
                self.clear_search();
                self.clear_selection();
            }
            KeyCode::Home => self.safe_move(isize::MIN),
            KeyCode::End => self.safe_move(isize::MAX),
            KeyCode::PageUp => self.safe_move(-10),
            KeyCode::PageDown => self.safe_move(10),
            KeyCode::Char(c) if c.is_numeric() => {
                let index = c.to_digit(10).unwrap() as usize - 1;
                self.set_sort(index);
                self.do_sort();
            }
            KeyCode::Char(' ') => {
                if let Some(selected) = self.table_state.selected() {
                    if self.selected.contains(&selected) {
                        self.selected.retain(|&x| x != selected);
                    } else {
                        self.selected.push(selected);
                    }
                    self.safe_move(1);
                }
            }
            KeyCode::Char('/') => self.searching = true,
            KeyCode::Char('a') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if self.get_selected().len() == self.filtered.len() {
                        self.clear_selection();
                    } else {
                        self.select_all();
                    }
                }
            }

            _ => {}
        }
        false
    }

    fn get_filter(&self) -> String {
        self.search_text_area.lines().join(" ")
    }

    pub fn focus(&mut self, focus: TableFocus) {
        self.focus_type = focus;
    }

    pub(crate) fn set_data(&mut self, rows: Vec<TableRow>) {
        //check if equal cannot just compare because of ordering
        if equal_unordered(
            rows.iter().map(|a| &a.cells).cloned().collect::<Vec<_>>(),
            self.data
                .iter()
                .map(|a| &a.cells)
                .cloned()
                .collect::<Vec<_>>(),
        ) {
            return;
        }

        //we save the selection values
        let previous_selection = self
            .selected
            .iter()
            .map(|&i| self.filtered[i].clone())
            .collect::<Vec<_>>();
        self.clear_selection();

        self.data = rows;
        self.filtered = self.data.clone();
        if self.filtered.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
        self.do_sort();

        //now restore selection if it exists in new dataset
        for row in previous_selection {
            if let Some((i, _)) = self.filtered.iter().enumerate().find(|(_, r)| *r == &row) {
                self.selected.push(i);
            }
        }
        self.update_filtered();
    }

    fn safe_move(&mut self, change: isize) {
        if self.filtered.is_empty() {
            return;
        }

        let len = self.filtered.len();
        let tstate = &mut self.table_state;
        if change < 0 {
            tstate.select(
                tstate
                    .selected()
                    .map(|s| s.saturating_sub(change.unsigned_abs())),
            );
        } else {
            tstate.select(
                tstate
                    .selected()
                    .map(|s| (s.saturating_add(change as usize)).min(len - 1)),
            );
        }
    }
    fn set_sort(&mut self, column_index: usize) {
        if column_index >= self.columns.len() {
            return;
        }

        if self.sort_by.0 == column_index {
            if self.sort_by.1 == Sort::Asc {
                self.sort_by.1 = Sort::Desc;
            } else {
                self.sort_by.1 = Sort::Asc;
            }
        } else {
            self.sort_by.0 = column_index;
            self.sort_by.1 = Sort::Asc;
        }
    }
    fn do_sort(&mut self) {
        let sort_col = self.sort_by.0;
        let sort_dir = self.sort_by.1;

        //strict sort
        /*
        match sort_dir {
            Sort::Asc => self.data.sort_by(|a, b| a[sort_col].cmp(&b[sort_col])),
            Sort::Desc => self.data.sort_by(|a, b| b[sort_col].cmp(&a[sort_col])),
        }*/
        //natural sort
        match sort_dir {
            Sort::Asc => self
                .filtered
                .sort_by(|a, b| natural_cmp(&a.cells[sort_col], &b.cells[sort_col])),
            Sort::Desc => self
                .filtered
                .sort_by(|a, b| natural_cmp(&b.cells[sort_col], &a.cells[sort_col])),
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn get_selected(&self) -> Vec<&TableRow> {
        self.filtered
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected.contains(i))
            .map(|(_, r)| r)
            .collect::<Vec<&TableRow>>()
    }

    pub(crate) fn select_all(&mut self) {
        self.selected = (0..self.filtered.len()).collect();
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = Some(title.to_string());
    }

    pub(crate) fn set_current(&mut self, new_index: Option<usize>) {
        self.table_state.select(new_index)
    }

    pub(crate) fn rows(&self) -> &Vec<TableRow> {
        &self.filtered
    }

    pub fn clear_search(&mut self) {
        self.search_text_area.select_all();
        self.search_text_area.cut();
        self.update_filtered();
    }
    fn update_filtered(&mut self) {
        let old_selected = self
            .table_state
            .selected()
            .and_then(|i| self.filtered.get(i).cloned());

        let filter = self.get_filter();
        if filter.is_empty() {
            self.filtered = self.data.clone();
        } else {
            self.filtered = self
                .data
                .iter()
                .filter(|row| {
                    row.cells
                        .iter()
                        .any(|cell| cell.to_lowercase().contains(&filter.to_lowercase()))
                })
                .cloned()
                .collect();
        }
        //try find old selected in new filtered
        if let Some(old_selected) = old_selected
            && let Some((i, _)) = self
                .filtered
                .iter()
                .enumerate()
                .find(|(_, r)| *r == &old_selected)
        {
            self.table_state.select(Some(i));
            return;
        }
        //else select first
        if self.filtered.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    pub(crate) fn get_current(&self) -> Option<&TableRow> {
        self.table_state
            .selected()
            .and_then(|i| self.filtered.get(i))
    }
}

fn equal_unordered(mut a: Vec<Vec<String>>, mut b: Vec<Vec<String>>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.sort_unstable();
    b.sort_unstable();
    a == b
}

impl Widget for TableWidget {
    fn render(mut self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let (current_fg, current_bg) = match self.focus_type {
            TableFocus::Focused => (Color::Black, Color::Yellow),
            TableFocus::UnfocusedDimmed => (Color::Black, Color::DarkGray),
            TableFocus::Unfocused => (Color::White, Color::Reset),
        };

        let selected_colour = match self.focus_type {
            TableFocus::Focused => Color::LightBlue,
            TableFocus::UnfocusedDimmed => Color::Gray,
            TableFocus::Unfocused => Color::Gray,
        };

        let footer = if self.selected.is_empty() {
            String::new()
        } else {
            format!("{} selected", self.selected.len())
        };
        let block = Block::bordered()
            .title(self.title.clone().unwrap_or_default())
            .title_bottom(Line::from(footer).bg(selected_colour).black().underlined());

        let mut table = Table::new(
            self.filtered.iter().enumerate().map(|(ri, item)| {
                let mut row = Row::new(item.cells.iter().map(|c| c.as_str()));
                if self.selected.contains(&ri) {
                    row = row.bg(selected_colour).fg(Color::Black).underlined();
                } else if let Some(col) = item.highlight {
                    row = row.fg(col);
                }
                row
            }),
            self.widths,
        )
        .row_highlight_style(Style::new().bg(current_bg).fg(current_fg))
        .block(block);
        if !self.columns.is_empty() {
            table = table.header(
                self.columns
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, c)| {
                        let c = match self.sort_by.1 {
                            Sort::Asc if self.sort_by.0 == i => format!("{} ↑", c),
                            Sort::Desc if self.sort_by.0 == i => format!("{} ↓", c),
                            _ => c,
                        }
                        .to_string();
                        Cell::from(c).black()
                    })
                    .collect::<Row>()
                    .bold()
                    .bg(Color::Red),
            )
        }
        <Table as StatefulWidget>::render(table, area, buf, &mut self.table_state);

        let top_right = ratatui::layout::Rect {
            x: area.x + area.width.saturating_sub(21),
            y: area.y,
            width: 20,
            height: 1,
        };

        draw_search(&mut self.search_text_area, top_right, buf, self.searching);
    }
}

fn draw_search(
    search_text_area: &mut TextArea<'static>,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    searching: bool,
) {
    if !searching && search_text_area.is_empty() {
        return;
    }
    Clear.render(area, buf);

    if searching {
        search_text_area.set_cursor_style(Style::default().bg(Color::White));
        search_text_area.set_style(Style::default().bg(Color::Blue).fg(Color::Black));
        search_text_area.render(area, buf);
    } else if !search_text_area.is_empty() {
        search_text_area.set_cursor_style(Style::default());
        search_text_area.set_style(Style::default().bg(Color::Gray).fg(Color::Black));
        search_text_area.render(area, buf);
    }
}

fn get_textarea() -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_placeholder_text("Search...");
    textarea.set_style(Style::default().bg(Color::Blue).fg(Color::Black));
    textarea.set_placeholder_style(Style::default().bg(Color::Blue).fg(Color::DarkGray));

    textarea
}
