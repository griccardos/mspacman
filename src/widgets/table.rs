use std::{cmp::Ordering, error::Error, isize};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Constraint,
    style::{Color, Style, Stylize},
    widgets::{Cell, Row, StatefulWidget, Table, TableState, Widget},
};
use regex::Regex;

use crate::structs::EventResult;

#[derive(Default, Debug, Clone)]
pub struct TableWidget {
    columns: Vec<String>,
    widths: Vec<Constraint>,
    data: Vec<Vec<String>>,
    table_state: TableState,
    sort_by: (usize, Sort),
    selected: Vec<usize>,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Sort {
    #[default]
    Asc,
    Desc,
}

impl TableWidget {
    pub fn new(columns: Vec<String>, widths: Vec<Constraint>) -> Self {
        Self {
            columns,
            widths,
            data: vec![],
            table_state: TableState::default(),
            sort_by: (0, Sort::Asc),
            selected: vec![],
        }
    }

    pub(crate) fn handle_key_event(
        &mut self,
        key: &KeyEvent,
    ) -> Result<EventResult, Box<dyn Error>> {
        //self.columns[0] = format!("{}", key.code);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.safe_move(-1),
            KeyCode::Down | KeyCode::Char('j') => self.safe_move(1),
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
                }
            }
            _ => {}
        }
        Ok(EventResult::None)
    }

    pub(crate) fn set_data(&mut self, filtered: Vec<Vec<String>>) {
        if filtered == self.data {
            return;
        }

        //we save the selection values
        let previous_selection = self
            .selected
            .iter()
            .map(|&i| self.data[i].clone())
            .collect::<Vec<Vec<String>>>();
        self.clear_selection();

        self.data = filtered;
        if self.data.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
        self.do_sort();

        //now restore selection if it exists in new dataset
        for row in previous_selection {
            if let Some((i, _)) = self.data.iter().enumerate().find(|(_, r)| *r == &row) {
                self.selected.push(i);
            }
        }
    }

    fn safe_move(&mut self, change: isize) {
        if self.data.is_empty() {
            return;
        }
        let len = self.data.len();
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
                .data
                .sort_by(|a, b| natural_cmp(&a[sort_col], &b[sort_col])),
            Sort::Desc => self
                .data
                .sort_by(|a, b| natural_cmp(&b[sort_col], &a[sort_col])),
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn get_selected_indices(&self) -> &Vec<usize> {
        &self.selected
    }

    pub(crate) fn select_all(&mut self) {
        self.selected = (0..self.data.len()).collect();
    }
}

impl Widget for TableWidget {
    fn render(mut self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let table = Table::new(
            self.data.iter().enumerate().map(|(ri, item)| {
                let mut row = Row::new(item.iter().map(|c| c.as_str()));
                if self.selected.contains(&ri) {
                    row = row.style(Style::new().bg(Color::Blue).fg(Color::Black).underlined());
                }
                row
            }),
            self.widths,
        )
        .header(
            self.columns
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, c)| {
                    let style = if self.sort_by.0 == i {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    let c = match self.sort_by.1 {
                        Sort::Asc if self.sort_by.0 == i => format!("{} ↑", c),
                        Sort::Desc if self.sort_by.0 == i => format!("{} ↓", c),
                        _ => c,
                    };
                    let c = format!("{}) {}", i + 1, c);
                    Cell::from(c).style(style)
                })
                .collect::<Row>()
                .style(Style::default().underlined().bold()),
        )
        .row_highlight_style(Style::new().bg(Color::Yellow).fg(Color::Black));
        <Table as StatefulWidget>::render(table, area, buf, &mut self.table_state);
    }
}

///natural sort comparison of two strings
///sort by numbers within strings
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let re = Regex::new(r"\d+|\D+").unwrap();
    let mut ai = re.find_iter(a);
    let mut bi = re.find_iter(b);

    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(am), Some(bm)) => {
                let as_ = am.as_str();
                let bs_ = bm.as_str();

                let a_is_num = as_
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false);
                let b_is_num = bs_
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false);

                if a_is_num && b_is_num {
                    // compare numerically (handles multi-digit numbers)
                    let an = as_.parse::<u64>().unwrap_or(0);
                    let bn = bs_.parse::<u64>().unwrap_or(0);
                    match an.cmp(&bn) {
                        Ordering::Equal => {
                            // tie-breaker: shorter numeric token (so "01" < "1" if you want),
                            // or you can skip this and treat them equal.
                            match as_.len().cmp(&bs_.len()) {
                                Ordering::Equal => continue,
                                ord => return ord,
                            }
                        }
                        ord => return ord,
                    }
                } else {
                    // lexicographic (case-sensitive). For case-insensitive use .to_lowercase()
                    match as_.cmp(bs_) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
            }
        }
    }
}
