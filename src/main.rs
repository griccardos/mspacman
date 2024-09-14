pub mod structs;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    DefaultTerminal, Frame,
};
use std::{collections::HashMap, error::Error, isize, process::Command};
use structs::{AppState, Focus, Package, Reason, Sort};
use tui_textarea::TextArea;

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
        sort_by: (1, Sort::Asc),
        hide_columns: HashMap::from_iter([(2, false), (3, false), (4, false), (5, false)]),
        ..Default::default()
    };
    state.left_table_state.select(Some(0));
    state.centre_table_state.select(Some(0));
    state.right_table_state.select(Some(0));

    let mut terminal = ratatui::init();
    terminal.clear()?;
    let appresult = run(&mut terminal, state);
    ratatui::restore();

    match appresult {
        Ok(selected) => selected.iter().for_each(|f| println!("{f}")),
        Err(e) => eprintln!("{e}"),
    }
    Ok(())
}

fn run(terminal: &mut DefaultTerminal, mut state: AppState) -> Result<Vec<String>, Box<dyn Error>> {
    let mut textarea = get_textarea();

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
                .constraints(Constraint::from_percentages([20, 60, 20]))
                .split(body_status[0]);

            draw_dependencies(&mut state, f, body[0]).unwrap();
            draw_centre(&mut state, f, body[1]).unwrap();
            draw_dependents(&mut state, f, body[2]).unwrap();
            draw_info(&mut state, f, body_status[1]).unwrap();
            draw_status(&mut state, f, body_status[2], &mut textarea).unwrap();
        })?;
        let must_quit = handle_event(&mut state, &mut textarea)?;
        if must_quit {
            break;
        }
    }
    Ok(state.selected)
}

fn get_textarea() -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_placeholder_text("Search");
    textarea.set_style(Style::default().bg(Color::Blue).fg(Color::Black));
    textarea.set_placeholder_style(Style::default().bg(Color::Blue).fg(Color::Gray));
    textarea
}

fn handle_event(state: &mut AppState, textarea: &mut TextArea) -> Result<bool, Box<dyn Error>> {
    let clear_textarea = |state: &mut AppState, textarea: &mut TextArea| {
        textarea.select_all();
        textarea.cut();
        state.filter = String::new();
        update_filter(state);
    };
    if let Event::Key(key) = event::read()? {
        //if searching, we handle input and return
        if state.searching {
            match key.code {
                KeyCode::Esc => {
                    state.searching = false;
                    clear_textarea(state, textarea);
                    return Ok(false);
                }
                KeyCode::Enter => {
                    state.searching = false;
                    return Ok(false);
                }
                _ => {}
            }

            textarea.input(key);
            state.filter = textarea.lines().join(" ");
            update_filter(state);

            return Ok(false);
        }

        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Esc => {
                    clear_textarea(state, textarea);
                }
                KeyCode::Char('c') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        state.selected.clear();
                        return Ok(true);
                    }
                }
                KeyCode::Char(val) if val >= '1' && val <= '5' => {
                    let val = val.to_digit(10).unwrap() as usize;
                    if key.modifiers.contains(KeyModifiers::ALT) {
                        hide_column(state, val)
                    } else {
                        sort_column(state, val)
                    }
                }

                KeyCode::Char('e') => {
                    state.only_expl = !state.only_expl;
                    update_filter(state);
                }
                KeyCode::Char('o') => {
                    state.only_orphans = !state.only_orphans;
                    update_filter(state);
                }

                KeyCode::Char('f') => {
                    state.only_foreign = !state.only_foreign;
                    update_filter(state);
                }
                KeyCode::Char('i') => state.show_info = !state.show_info,
                KeyCode::Char('/') => state.searching = true,
                KeyCode::Down => safe_move(state, 1),
                KeyCode::Up => safe_move(state, -1),
                KeyCode::PageDown => safe_move(state, 10),
                KeyCode::PageUp => safe_move(state, -10),
                KeyCode::Home => safe_move(state, -isize::MAX),
                KeyCode::End => safe_move(state, isize::MAX),
                KeyCode::Left => cycle_focus(state, -1),
                KeyCode::Right => cycle_focus(state, 1),
                KeyCode::Enter => handle_enter(state),
                KeyCode::Char(' ') => handle_select(state),
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

fn hide_column(state: &mut AppState, arg: usize) {
    state.hide_columns.entry(arg).and_modify(|a| *a = !*a);
}
fn sort_column(state: &mut AppState, arg: usize) {
    if state.sort_by.0 == arg {
        if state.sort_by.1 == Sort::Asc {
            state.sort_by.1 = Sort::Desc;
        } else {
            state.sort_by.1 = Sort::Asc;
        }
    } else {
        state.sort_by.0 = arg;
        state.sort_by.1 = Sort::Asc;
    }
    let sort_col = state.sort_by.0;
    let sort_dir = state.sort_by.1;

    //always sort by name first
    state.filtered.sort_by(|a, b| a.name.cmp(&b.name));
    match sort_col {
        1 => {
            if sort_dir == Sort::Asc {
                state.filtered.sort_by(|a, b| a.name.cmp(&b.name));
            } else {
                state.filtered.sort_by(|a, b| b.name.cmp(&a.name));
            }
        }
        2 => {
            if sort_dir == Sort::Asc {
                state
                    .filtered
                    .sort_by(|a, b| a.reason.partial_cmp(&b.reason).unwrap());
            } else {
                state
                    .filtered
                    .sort_by(|a, b| b.reason.partial_cmp(&a.reason).unwrap());
            }
        }
        3 => {
            if sort_dir == Sort::Asc {
                state
                    .filtered
                    .sort_by(|a, b| a.required_by.len().cmp(&b.required_by.len()));
            } else {
                state
                    .filtered
                    .sort_by(|a, b| b.required_by.len().cmp(&a.required_by.len()));
            }
        }
        4 => {
            if sort_dir == Sort::Asc {
                state.filtered.sort_by(|a, b| a.validated.cmp(&b.validated));
            } else {
                state.filtered.sort_by(|a, b| b.validated.cmp(&a.validated));
            }
        }
        5 => {
            if sort_dir == Sort::Asc {
                state.filtered.sort_by(|a, b| a.installed.cmp(&b.installed));
            } else {
                state.filtered.sort_by(|a, b| b.installed.cmp(&a.installed));
            }
        }

        _ => {}
    }
}

fn update_filter(state: &mut AppState) {
    state.filtered = state
        .packs
        .iter()
        .filter(|p| (state.only_expl && p.reason == Reason::Explicit) || !state.only_expl) //only show explicit packages
        .filter(|p| (state.only_foreign && !p.validated) || !state.only_foreign) //only show foreign packages
        .filter(|p| {
            (state.only_orphans
                && p.required_by.is_empty()
                && p.optional_for.is_empty()
                && p.reason == Reason::Dependency)
                || !state.only_orphans
        }) //only show orphans
        .filter(|p| p.name.contains(&state.filter))
        .cloned()
        .collect();
}

fn handle_select(state: &mut AppState) {
    if state.focus != Focus::Centre {
        return;
    }
    let pack = current_pack(&state);
    if pack.is_none() {
        return;
    }
    let pack = pack.unwrap();
    let name = pack.name.clone();

    if state.selected.contains(&name) {
        state.selected.retain(|p| p != &name);
    } else {
        state.selected.push(name);
    }
}

fn handle_enter(state: &mut AppState) {
    let pack = current_pack(&state);
    if pack.is_none() {
        return;
    }
    let pack = pack.unwrap();
    let name = match state.focus {
        Focus::Left => pack
            .dependencies
            .get(state.left_table_state.selected().unwrap())
            .cloned()
            .unwrap_or_default(),
        Focus::Centre => return,
        Focus::Right => pack
            .required_by
            .get(state.right_table_state.selected().unwrap())
            .cloned()
            .unwrap_or_default(),
    };

    if let Some(pack) = get_pack(&state, &name).cloned() {
        //undo any filters
        state.only_expl = false;
        state.filtered = state.packs.clone();
        if let Some(prevpack) = current_pack(&state) {
            let prev = prevpack.name.clone();
            goto_package(state, &pack.name.clone());
            state.prev.push(prev);
        }
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
    if pack.is_none() {
        return;
    }
    let pack = pack.unwrap();
    let left_count = pack.dependencies.len();
    let right_count = pack.required_by.len();
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

fn draw_info(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    //info
    let pack = current_pack(&state);
    if pack.is_none() {
        return Ok(());
    }
    let pack = pack.unwrap();
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
fn draw_status(
    state: &mut AppState,
    f: &mut Frame,
    rect: Rect,
    textarea: &mut TextArea,
) -> Result<(), Box<dyn Error>> {
    let mut text = vec![
        "q:Quit",
        "i:Info",
        "d:Date",
        "e:Expl",
        "f:Foreign",
        "SPC:Select",
        "Alt+[2-5]:Minimize",
    ];
    let sname = match state.sort_by.0 {
        1 => "Name",
        2 => "Reason",
        3 => "Required By",
        4 => "Foreign",
        5 => "Installed",
        _ => "",
    };
    let sname = format!("[1-5]:Sort [{sname} {:?}]", state.sort_by.1);
    text.push(&sname);

    if state.prev.len() > 0 {
        text.push("BSP: Back");
    }

    if state.only_expl {
        text.push("Showing only Explicitly Installed");
    }
    if !state.message.is_empty() {
        text.push(&state.message);
    }

    let search_len = if state.searching || !textarea.is_empty() {
        textarea
            .lines()
            .get(0)
            .map(|l| l.len() + 1)
            .unwrap_or(0)
            .max(8) as u16
    } else {
        0
    };

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(search_len), Constraint::Min(0)].as_ref())
        .split(rect);
    if state.searching || !textarea.is_empty() {
        if state.searching {
            textarea.set_cursor_style(Style::default().bg(Color::White));
        } else {
            textarea.set_cursor_style(Style::default().bg(Color::Blue));
        }
        f.render_widget(&*textarea, layout[0]);
    }

    let para = Paragraph::new(text.join(" ")).style(Style::default().fg(Color::Yellow));
    f.render_widget(&para, layout[1]);
    Ok(())
}
fn safe_move(state: &mut AppState, change: isize) {
    let pack = current_pack(&state);
    if pack.is_none() {
        return;
    }
    let pack = pack.unwrap();
    let len = match &state.focus {
        Focus::Left => pack.dependencies.len(),
        Focus::Centre => state.filtered.len(),
        Focus::Right => pack.required_by.len(),
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

fn current_pack(state: &AppState) -> Option<&Package> {
    state
        .filtered
        .get(state.centre_table_state.selected().unwrap_or_default())
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
            let data = vec![
                pack.name.clone(),
                format!("{:?}", pack.reason),
                pack.required_by.len().to_string(),
                format!("{}", if pack.validated { "" } else { "X" }),
                pack.installed.clone(),
            ];

            let mut row = Row::from_iter(data);
            let mut style = Style::default();
            if pack.reason == Reason::Explicit {
                style = style.fg(Color::Green);
            }
            if state.selected.contains(&pack.name) {
                style = style.underlined();
            }
            row = row.style(style);

            row
        })
        .collect();

    let style = if state.focus == Focus::Centre {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default().bg(Color::Gray).fg(Color::Black)
    };
    let head = vec!["Name", "Reason", "ReqBy", "Foreign", "Installed"];

    let mut widths = vec![
        Constraint::Percentage(50),
        Constraint::Percentage(15),
        Constraint::Min(5),
        Constraint::Min(3),
        Constraint::Length(19),
    ];

    for (i, c) in widths.iter_mut().enumerate() {
        if let Some(v) = state.hide_columns.get(&(i + 1)) {
            if *v {
                *c = Constraint::Length(1);
            }
        }
    }

    let table = Table::new(rows, widths)
        .header(
            head.into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .style(Style::default().underlined().bold()),
        )
        .highlight_style(style);
    let count = state.filtered.len();
    let local = state.filtered.iter().filter(|p| p.validated).count();
    let foreign = count - local;
    let extra = if foreign > 0 {
        format!(" ({local} pacman, {foreign} foreign)")
    } else {
        "".to_string()
    };
    let block = Block::default()
        .title(format!("Installed {count}{extra}"))
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
    if pack.is_none() {
        return Ok(());
    }
    let pack = pack.unwrap();
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
    if pack.is_none() {
        return Ok(());
    }
    let pack = pack.unwrap();
    let count = pack.required_by.len();
    let rows: Vec<Row> = pack
        .required_by
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
                pack.required_by = value
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .filter(|r| r != "None")
                    .collect()
            }
            "Optional For" => {
                pack.optional_for = value
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
            "Install Date" => pack.installed = to_date(value),
            "Description" => pack.description = value.to_string(),
            "Validated By" => pack.validated = value == "Signature",
            _ => {}
        }
    }
    packs.push(pack);
    Ok(packs)
}

fn to_date(value: &str) -> String {
    //get rid of the timezone
    let value = value.rsplit_once(' ').unwrap().0;
    let time = jiff::fmt::strtime::parse("%a %d %b %Y %I:%M:%S %p", value).unwrap();
    time.to_datetime().unwrap().to_string().replace("T", " ")
    //time.to_string("%Y-%m-%d %H:%M:%S").unwrap()
}
