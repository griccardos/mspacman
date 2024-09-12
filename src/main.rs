use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    DefaultTerminal, Frame,
};
use std::{error::Error, isize, process::Command};

#[derive(Debug, Default)]
struct AppState {
    packs: Vec<Package>,
    filtered: Vec<Package>,
    centre_table_state: TableState,
    left_table_state: TableState,
    right_table_state: TableState,
    focus: Focus,
    sort: Sort,
    prev: Vec<String>,
    only_expl: bool,
    show_info: bool,
    message: String,
}
#[derive(Debug, Default, PartialEq, Clone, Copy)]
enum Focus {
    Left,
    #[default]
    Centre,
    Right,
}

#[derive(Debug, Default)]
enum Sort {
    #[default]
    Name,
    NameDesc,
    Dependents,
    DependentsDesc,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !pacman_exists() {
        println!("pacman is not installed");
        std::process::exit(1);
    }

    let packs = get_packs()?;
    let mut state = AppState {
        filtered: packs.clone(),
        packs,
        show_info: true,
        ..Default::default()
    };
    state.left_table_state.select(Some(0));
    state.centre_table_state.select(Some(0));
    state.right_table_state.select(Some(0));

    let mut terminal = ratatui::init();
    terminal.clear().unwrap();
    let appresult = run(terminal, state);
    ratatui::restore();

    appresult
}

fn run(
    mut terminal: DefaultTerminal,
    mut state: AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| {
            let info = if state.show_info { 5 } else { 0 };

            let body_status = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(100),
                    Constraint::Min(info),
                    Constraint::Min(1),
                ])
                .split(f.area());
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(Constraint::from_percentages([25, 45, 25]))
                .split(body_status[0]);

            draw_dependencies(&mut state, f, body[0]).unwrap();
            draw_centre(&mut state, f, body[1]).unwrap();
            draw_dependents(&mut state, f, body[2]).unwrap();
            draw_info(&mut state, f, body_status[1]).unwrap();
            draw_status(&mut state, f, body_status[2]).unwrap();
        })?;
        let must_quit = handle_event(&mut state)?;
        if must_quit {
            break;
        }
    }
    Ok(())
}

fn handle_event(state: &mut AppState) -> Result<bool, Box<dyn Error>> {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Char('c') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        return Ok(true);
                    }
                }
                KeyCode::Char('s') => cycle_sort(state),
                KeyCode::Char('e') => {
                    state.only_expl = !state.only_expl;
                    if state.only_expl {
                        state.filtered = state
                            .packs
                            .iter()
                            .filter(|p| p.reason == Reason::Explicit)
                            .cloned()
                            .collect();
                    } else {
                        state.filtered = state.packs.clone()
                    };
                }
                KeyCode::Char('i') => state.show_info = !state.show_info,
                KeyCode::Esc => return Ok(true),
                KeyCode::Down => safe_move(state, 1),
                KeyCode::Up => safe_move(state, -1),
                KeyCode::PageDown => safe_move(state, 10),
                KeyCode::PageUp => safe_move(state, -10),
                KeyCode::Home => safe_move(state, -isize::MAX),
                KeyCode::End => safe_move(state, isize::MAX),
                KeyCode::Left => cycle_focus(state, -1),
                KeyCode::Right => cycle_focus(state, 1),
                KeyCode::Enter => handle_enter(state),
                KeyCode::Backspace => {
                    if let Some(prev) = state.prev.pop() {
                        goto_package(state, &prev);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(false)
}

fn handle_enter(state: &mut AppState) {
    let pack = current_pack(&state);
    let name = match state.focus {
        Focus::Left => pack
            .dependencies
            .get(state.left_table_state.selected().unwrap())
            .cloned()
            .unwrap_or_default(),
        Focus::Centre => return,
        Focus::Right => pack
            .dependents
            .get(state.right_table_state.selected().unwrap())
            .cloned()
            .unwrap_or_default(),
    };

    if let Some(pack) = get_pack(&state, &name).cloned() {
        //undo any filters
        state.only_expl = false;
        state.filtered = state.packs.clone();

        let prev = current_pack(&state).name.clone();
        goto_package(state, &pack.name.clone());
        state.prev.push(prev);
    }
}

fn goto_package(state: &mut AppState, name: &str) {
    let new_index = state
        .filtered
        .iter()
        .position(|p| &p.name == name)
        .unwrap_or_default();
    state.focus = Focus::Centre;
    state.centre_table_state.select(Some(new_index));
}

fn cycle_focus(state: &mut AppState, arg: i32) {
    let pack = current_pack(&state);
    let left_count = pack.dependencies.len();
    let right_count = pack.dependents.len();
    state.focus = if arg > 0 {
        match state.focus {
            Focus::Left => Focus::Centre,
            Focus::Centre if right_count > 0 => Focus::Right,
            _ => state.focus, //else keep
        }
    } else {
        match state.focus {
            Focus::Centre if left_count > 0 => Focus::Left,
            Focus::Right => Focus::Centre,
            _ => state.focus, //else keep
        }
    };
    match state.focus {
        Focus::Left => state.left_table_state.select(Some(0)),
        Focus::Centre => {}
        Focus::Right => state.right_table_state.select(Some(0)),
    }
}

fn cycle_sort(state: &mut AppState) {
    match state.sort {
        Sort::Name => state.sort = Sort::NameDesc,
        Sort::NameDesc => state.sort = Sort::Dependents,
        Sort::Dependents => state.sort = Sort::DependentsDesc,
        Sort::DependentsDesc => state.sort = Sort::Name,
    }
    match state.sort {
        Sort::Name => state.filtered.sort_by(|a, b| a.name.cmp(&b.name)),
        Sort::NameDesc => state.filtered.sort_by(|a, b| b.name.cmp(&a.name)),
        Sort::Dependents => {
            state.filtered.sort_by(|a, b| a.name.cmp(&b.name)); //first sort by name
            state
                .filtered
                .sort_by(|a, b| a.dependents.len().cmp(&b.dependents.len()));
        }
        Sort::DependentsDesc => state
            .filtered
            .sort_by(|a, b| b.dependents.len().cmp(&a.dependents.len())),
    }
}
fn draw_info(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    //info
    let pack = current_pack(&state);
    let rows: Vec<Row> = [
        ("Version", pack.version.as_str()),
        ("Installed", pack.installed.as_str()),
        ("Description", pack.description.as_str()),
    ]
    .iter()
    .map(|s| Row::from_iter([s.0, s.1]))
    .collect();
    let table = Table::new(rows, [Constraint::Length(15), Constraint::Min(0)])
        .block(Block::bordered().title("Info"));
    if state.show_info {
        f.render_widget(table, rect);
    }
    Ok(())
}
fn draw_status(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    let mut text = vec!["q: Quit", "i: Info", "e: Expl"];

    match state.sort {
        Sort::Name => text.push("s: Sort [Name]"),
        Sort::NameDesc => text.push("s: Sort [Name Desc]"),
        Sort::Dependents => text.push("s: Sort [Dependents]"),
        Sort::DependentsDesc => text.push("s: Sort [Dependents Desc]"),
    }
    if state.prev.len() > 0 {
        text.push("BSP: Back");
    }

    if state.only_expl {
        text.push("Showing only Explicitly Installed");
    }
    if !state.message.is_empty() {
        text.push(&state.message);
    }

    let para = Paragraph::new(text.join(" ")).style(Style::default().fg(Color::Yellow));
    f.render_widget(&para, rect);
    Ok(())
}
fn safe_move(state: &mut AppState, change: isize) {
    let pack = current_pack(&state);
    let len = match &state.focus {
        Focus::Left => pack.dependencies.len(),
        Focus::Centre => state.filtered.len(),
        Focus::Right => pack.dependents.len(),
    };
    let tstate = match state.focus {
        Focus::Left => &mut state.left_table_state,
        Focus::Centre => &mut state.centre_table_state,
        Focus::Right => &mut state.right_table_state,
    };

    if change < 0 {
        tstate.select(
            tstate
                .selected()
                .map(|s| s.saturating_sub(change.abs() as usize)),
        );
    } else {
        tstate.select(
            tstate
                .selected()
                .map(|s| (s + change as usize).min(len - 1)),
        );
    }
}

fn current_pack(state: &AppState) -> &Package {
    let pack = state
        .filtered
        .get(state.centre_table_state.selected().unwrap_or_default())
        .unwrap_or(state.filtered.first().unwrap());
    pack
}
fn get_pack<'a, 'b>(state: &'a AppState, name: &'b str) -> Option<&'a Package> {
    let pack = state.packs.iter().find(|p| p.name == name);
    pack
}

///draws list, plus package info
fn draw_centre(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    let rows: Vec<Row> = state
        .filtered
        .iter()
        .map(|pack| {
            let mut row = Row::from_iter([
                pack.name.clone(),
                format!("{:?}", pack.reason),
                pack.dependents.len().to_string(),
            ]);
            if pack.reason == Reason::Explicit {
                row = row.style(Style::default().fg(Color::Green));
            }

            row
        })
        .collect();

    let style = if state.focus == Focus::Centre {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default().bg(Color::Gray).fg(Color::Black)
    };
    let table = Table::new(rows, Constraint::from_percentages([65, 25, 10]))
        .header(
            ["Name", "Reason", "Req"]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .style(Style::default().underlined().on_red()),
        )
        .highlight_style(style);
    let count = state.filtered.len();
    let block = Block::default()
        .title(format!("Installed {count} "))
        .borders(Borders::all());
    let table = table.block(block);
    f.render_stateful_widget(&table, rect, &mut state.centre_table_state);

    Ok(())
}

fn draw_dependencies(
    state: &mut AppState,
    f: &mut Frame,
    rect: Rect,
) -> Result<(), Box<dyn Error>> {
    let pack = current_pack(&state);
    let rows: Vec<Row> = pack
        .dependencies
        .iter()
        .map(|dep| {
            let mut style = Style::default();
            if let Some(p) = get_pack(state, dep) {
                if p.reason == Reason::Explicit {
                    style = style.fg(Color::Green);
                }
            } else {
                style = style.fg(Color::Red);
            }
            Row::new(vec![Cell::from(dep.clone())]).style(style)
        })
        .collect();
    let count = pack.dependencies.len();
    let style = if state.focus == Focus::Left {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default()
    };
    let table = Table::new(rows, [Constraint::Min(0)]).highlight_style(style);
    let block = Block::default()
        .title(format!("Depends on {count}"))
        .borders(Borders::all());
    let table = table.block(block);
    f.render_stateful_widget(&table, rect, &mut state.left_table_state);
    Ok(())
}
fn draw_dependents(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    let pack = current_pack(&state);
    let count = pack.dependents.len();
    let rows: Vec<Row> = pack
        .dependents
        .iter()
        .map(|dep| Row::new(vec![Cell::from(dep.clone())]))
        .collect();
    let style = if state.focus == Focus::Right {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default()
    };

    let table = Table::new(rows, [Constraint::Min(0)]).highlight_style(style);
    let block = Block::default()
        .title(format!("Required by {count}"))
        .borders(Borders::all());
    let table = table.block(block);
    f.render_stateful_widget(&table, rect, &mut state.right_table_state);
    Ok(())
}

fn pacman_exists() -> bool {
    Command::new("pacman").output().is_ok()
}
#[derive(Debug, Default, Clone)]
struct Package {
    name: String,
    dependents: Vec<String>,
    dependencies: Vec<String>,
    reason: Reason,
    //info
    version: String,
    installed: String,
    description: String,
}
#[derive(Debug, Clone, Default, PartialEq)]
enum Reason {
    #[default]
    Dependency,
    Explicit,
    Other(String),
}

fn get_packs() -> Result<Vec<Package>, Box<dyn std::error::Error>> {
    let output = Command::new("pacman").arg("-Qi").output()?;
    let output = String::from_utf8(output.stdout)?;
    let mut packs: Vec<Package> = vec![];
    let mut pack = Package::default();

    for line in output.lines() {
        let pair = line.split_once(':');
        if pair.is_none() {
            continue;
        }
        let (key, value) = pair.unwrap();
        let key = key.trim();
        let value = value.trim();

        match key {
            "Name" => {
                if !pack.name.is_empty() {
                    packs.push(pack);
                    pack = Package::default();
                }
                pack.name = value.to_string()
            }
            "Version" => pack.version = value.to_string(),
            "Depends On" => {
                pack.dependencies = value
                    .split_whitespace()
                    .map(|r| r.to_string())
                    .filter(|r| r != "None")
                    .collect()
            }
            "Required By" => {
                pack.dependents = value
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .filter(|r| r != "None")
                    .collect()
            }
            "Install Reason" => {
                pack.reason = match value {
                    "Explicitly installed" => Reason::Explicit,
                    "Installed as a dependency for another package" => Reason::Dependency,
                    // _ => value.to_string(),
                    _ => Reason::Other(value.to_string()),
                }
            }
            "Install Date" => pack.installed = value.to_string(),
            "Description" => pack.description = value.to_string(),
            _ => {}
        }
    }
    packs.push(pack);
    Ok(packs)
}
