pub mod structs;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Offset, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    DefaultTerminal, Frame,
};
use std::{collections::HashMap, error::Error, io::Write, process::Command};
use structs::{AppState, EventResult, Focus, Package, Reason, Sort};
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
    let mut search_input = get_textarea("Search");

    loop {
        terminal.draw(|f| {
            let info = if state.show_info { 6 } else { 0 };
            let pr = if state.show_providing {
                ((f.area().height as f32 * 0.5) as u16).max(5)
            } else {
                0
            };

            let top_bottom = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(100),
                    Constraint::Min(info),
                    Constraint::Min(pr),
                    Constraint::Min(1),
                ])
                .split(f.area());
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(Constraint::from_percentages([20, 60, 20]))
                .split(top_bottom[0]);

            draw_dependencies(&mut state, f, body[0]).unwrap();
            draw_centre(&mut state, f, body[1]).unwrap();
            draw_dependents(&mut state, f, body[2]).unwrap();
            draw_info(&mut state, f, top_bottom[1]).unwrap();
            draw_provides(&mut state, f, top_bottom[2]).unwrap();
            draw_status(&mut state, f, top_bottom[3], &mut search_input).unwrap();
            draw_help(&mut state, f).unwrap();
            draw_command(&mut state, f).unwrap();
        })?;
        match handle_event(&mut state, &mut search_input)? {
            EventResult::None => {}
            EventResult::Quit => break,
            EventResult::Command(c) => {
                let _ = goto_screen(false, terminal);
                let res = run_command(&mut state, c);
                let _ = goto_screen(true, terminal);
                if let Err(e) = res {
                    state.message = e.to_string();
                }
            }
        }
    }
    Ok(state.selected)
}

fn get_textarea(label: &str) -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_placeholder_text(label);
    textarea.set_style(Style::default().bg(Color::Gray).fg(Color::Black));
    textarea.set_placeholder_style(Style::default().bg(Color::Gray).fg(Color::DarkGray));
    textarea
}

fn handle_event(
    state: &mut AppState,
    search_input: &mut TextArea,
) -> Result<EventResult, Box<dyn Error>> {
    let clear_search = |state: &mut AppState, textarea: &mut TextArea| {
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
                    clear_search(state, search_input);
                    return Ok(EventResult::None);
                }
                KeyCode::Enter => {
                    state.searching = false;
                    return Ok(EventResult::None);
                }
                _ => {}
            }

            search_input.input(key);
            state.filter = search_input.lines().join(" ");
            update_filter(state);

            return Ok(EventResult::None);
        }
        //if searching, we handle input and return
        if state.show_command {
            state.show_command = false;
            match key.code {
                KeyCode::Char('r') => return Ok(EventResult::Command('r')),
                KeyCode::Char('q') => return Ok(EventResult::Command('q')),
                KeyCode::Char('s') => return Ok(EventResult::Command('s')),
                _ => {}
            }

            return Ok(EventResult::None);
        }

        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('q') => return Ok(EventResult::Quit),
                KeyCode::Esc => {
                    state.only_expl = false;
                    state.only_foreign = false;
                    state.only_orphans = false;
                    state.show_help = false;
                    state.show_command = false;
                    state.selected.clear();
                    clear_search(state, search_input);
                    update_filter(state);
                }
                KeyCode::Char('?') | KeyCode::Char('h') => state.show_help = !state.show_help,
                KeyCode::Char('c') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        state.selected.clear();
                        return Ok(EventResult::Quit);
                    } else if !state.selected.is_empty() {
                        state.show_command = true;
                    }
                }
                KeyCode::Char(val) if ('1'..='5').contains(&val) => {
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
                KeyCode::Char('p') => state.show_providing = !state.show_providing,
                KeyCode::Char('r') => state.packs = get_packs()?,
                KeyCode::Char('/') => state.searching = true,
                KeyCode::Down => safe_move(state, 1),
                KeyCode::Up => safe_move(state, -1),
                KeyCode::PageDown => safe_move(state, 10),
                KeyCode::PageUp => safe_move(state, -10),
                KeyCode::Home => safe_move(state, -isize::MAX),
                KeyCode::End => safe_move(state, isize::MAX),
                KeyCode::Left => cycle_focus_horiz(state, -1),
                KeyCode::Right => cycle_focus_horiz(state, 1),
                KeyCode::Enter => handle_enter(state),
                KeyCode::Tab => cycle_focus_vert(state),
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
    Ok(EventResult::None)
}

fn goto_screen(alternate: bool, terminal: &mut DefaultTerminal) -> Result<(), Box<dyn Error>> {
    use crossterm::terminal::EnterAlternateScreen;
    use crossterm::terminal::LeaveAlternateScreen;
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use crossterm::ExecutableCommand;
    use std::io::stdout;

    if alternate {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        terminal.clear()?;
    } else {
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
    }
    Ok(())
}

fn run_command(state: &mut AppState, char: char) -> Result<(), Box<dyn Error>> {
    if state.selected.is_empty() {
        return Ok(());
    }
    let comm_arg = match char {
        'r' => Some(("pacman", vec!["-R"])),
        'q' => Some(("pacman", vec!["-Qi"])),
        's' => Some(("pacman", vec!["-S"])),
        _ => None,
    };
    if let Some((comm, mut args)) = comm_arg {
        args.extend(state.selected.iter().map(|a| a.as_str()));

        //try run command as is
        let res = Command::new(comm).args(&args).status()?;

        if !res.success() {
            std::io::stdout().write_all("running sudo\n".as_bytes())?;
            //run as sudo
            args.insert(0, comm);
            args.insert(0, "-S");
            let res = Command::new("sudo").args(&args).status()?;
            if !res.success() {
                std::io::stdout().write_all("Failed to run command".as_bytes())?;
            }
        }
        std::io::stdout().write_all("\nPress enter to continue...".as_bytes())?;
        std::io::stdout().flush()?;
        crossterm::event::read()?;
    };

    let before = state.packs.len();
    state.packs = get_packs()?;
    update_filter(state);
    state.message = format!("{}->{}", before, state.packs.len());
    Ok(())
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
        .filter(|p| !state.only_expl || p.reason == Reason::Explicit) //only show explicit packages
        .filter(|p| !state.only_foreign || !p.validated) //only show foreign packages
        .filter(|p| {
            !state.only_orphans
                || p.required_by.is_empty()
                    && p.optional_for.is_empty()
                    && p.reason == Reason::Dependency
        }) //only show orphans
        .filter(|p| p.name.contains(&state.filter))
        .cloned()
        .collect();

    //if empty, we have no selection
    if state.filtered.is_empty() {
        state.centre_table_state.select(None);
    } else {
        //if we are more than the length, select the first
        if let Some(i) = state.centre_table_state.selected() {
            if i >= state.filtered.len() {
                state.centre_table_state.select(Some(0));
            }
        } else {
            state.centre_table_state.select(Some(0));
        }
    }
    state
        .selected
        .retain(|p| state.packs.iter().any(|f| f.name == *p));
}

fn handle_select(state: &mut AppState) {
    if state.focus != Focus::Centre {
        return;
    }
    let pack = current_pack(state);
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
    let pack = current_pack(state);
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
        Focus::Provides => return,
    };

    if let Some(pack) = get_pack(state, &name).cloned() {
        //undo any filters
        state.only_expl = false;
        state.filtered = state.packs.clone();
        if let Some(prevpack) = current_pack(state) {
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
        .position(|p| p.name == name)
        .unwrap_or_default();
    state.focus = Focus::Centre;
    state.centre_table_state.select(Some(new_index));
}

fn cycle_focus_vert(state: &mut AppState) {
    state.focus = match state.focus {
        Focus::Provides => Focus::Centre,
        _ => {
            if state.show_providing {
                Focus::Provides
            } else {
                Focus::Centre
            }
        }
    };
    if let Some(curr) = current_pack(state) {
        if state.focus == Focus::Provides
            && state.provides_table_state.selected().is_none()
            && !curr.provides.is_empty()
        {
            state.provides_table_state.select(Some(0));
        }
    }
}

fn cycle_focus_horiz(state: &mut AppState, arg: i32) {
    let pack = current_pack(state);
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
        Focus::Provides => {}
    }
}

fn draw_info(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    //info
    let pack = current_pack(state);
    if pack.is_none() {
        return Ok(());
    }
    let pack = pack.unwrap();
    let rows: Vec<Row> = [
        ("Name", pack.name.as_str()),
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

fn draw_command(state: &mut AppState, f: &mut Frame) -> Result<(), Box<dyn Error>> {
    if !state.show_command {
        return Ok(());
    }
    let size = f.area();
    // Calculate the block size (1/3 of the screen size)
    let block_width = (size.width / 3).max(50);
    let block_height = (size.height / 3).max(14);

    // Calculate the block position (centered)
    let block_x = (size.width - block_width) / 2;
    let block_y = (size.height - block_height) / 2;

    // Create a centered block
    let block = Block::default()
        .title("Run command on selection")
        .borders(Borders::ALL)
        .title_style(Color::Black)
        .style(Style::default().bg(Color::Blue));

    //break up list of packages into lines, with a max length of block_width
    let mut lines = vec![];
    let mut line = String::new();
    for word in &state.selected {
        if line.len() + word.len() + 1 > block_width as usize {
            lines.push(line.clone());
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    lines.push(line);
    let mut para_lines: Vec<Line> =
        vec![format!("{} packages selected", state.selected.len()).into()];
    para_lines.extend(
        lines
            .into_iter()
            .map(|s| Line::from(s).style(Style::default().fg(Color::Yellow))),
    );
    para_lines.extend(vec![
        "".into(),
        "Commands:".into(),
        "r: Remove".into(),
        "s: Sync/Update".into(),
        "q: Query".into(),
    ]);

    let paragraph = Paragraph::new(para_lines);
    let window_rect = Rect::new(block_x, block_y, block_width, block_height);
    let para_rect = window_rect.offset(Offset { y: 3, x: 1 });

    // Render the block
    f.render_widget(Clear, window_rect);
    f.render_widget(block, window_rect);
    f.render_widget(paragraph, para_rect);
    Ok(())
}
fn draw_help(state: &mut AppState, f: &mut Frame) -> Result<(), Box<dyn Error>> {
    if !state.show_help {
        return Ok(());
    }
    let size = f.area();

    // Calculate the block size (1/3 of the screen size)
    let block_width = (size.width / 3).max(50);
    let block_height = (size.height / 3).max(14);

    // Calculate the block position (centered)
    let block_x = (size.width - block_width) / 2;
    let block_y = (size.height - block_height) / 2;

    // Create a centered block
    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Blue));

    // Create a paragraph to display inside the block
    let paragraph = Paragraph::new(vec![
        "q: Quit".into(),
        "i: Info".into(),
        "p: Provides".into(),
        "d: Date".into(),
        "e: Explicitly installed only".into(),
        "f: Foreign packages only".into(),
        "o: Orphans only".into(),
        "/: Search".into(),
        "[2-5]: Sort".into(),
        "Alt+[2-5]: Minimize Column".into(),
        "Enter (on outer columns): Goto Package".into(),
        "Left/Right: Switch column".into(),
        "Tab: Jump to provides tab".into(),
        "Esc: Clear filters and selection".into(),
        "Space: Select/Deselect".into(),
        "c: Run command on selection".into(),
    ])
    .block(block)
    .alignment(Alignment::Left);
    let rect = Rect::new(block_x, block_y, block_width, block_height);
    // Render the block
    f.render_widget(Clear, rect);
    f.render_widget(paragraph, rect);

    Ok(())
}
fn draw_status(
    state: &mut AppState,
    f: &mut Frame,
    rect: Rect,
    textarea: &mut TextArea,
) -> Result<(), Box<dyn Error>> {
    let mut text = vec!["  ?: Help   "];
    let sname = match state.sort_by.0 {
        1 => "Name",
        2 => "Reason",
        3 => "Required By",
        4 => "Foreign",
        5 => "Installed",
        _ => "",
    };
    let sname = format!("Sort [{sname} {:?}]", state.sort_by.1);
    text.push(&sname);

    if !state.prev.is_empty() {
        text.push("BSP: Back");
    }
    let mut filters = vec![];
    if state.only_expl {
        filters.push("ExplicitlyInstalled")
    }
    if state.only_foreign {
        filters.push("Foreign");
    }
    if state.only_orphans {
        filters.push("Orphans");
    }
    let txt = format!("'{}'", state.filter);

    if !state.filter.is_empty() {
        filters.push(&txt);
    }
    let fil = format!("Filters [{}]", filters.join(", "));
    if !filters.is_empty() {
        text.push(&fil);
    }

    if !state.message.is_empty() {
        text.push(&state.message);
    }

    let search_len = if state.searching || !textarea.is_empty() {
        textarea
            .lines()
            .first()
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
            textarea.set_cursor_style(Style::default().bg(Color::Gray));
        }
        f.render_widget(&*textarea, layout[0]);
    }

    let para = Paragraph::new(text.join(" ")).style(Style::default().fg(Color::Yellow));
    f.render_widget(&para, layout[1]);
    Ok(())
}
fn safe_move(state: &mut AppState, change: isize) {
    let pack = current_pack(state);
    if pack.is_none() {
        return;
    }
    let pack = pack.unwrap();
    let len = match &state.focus {
        Focus::Left => pack.dependencies.len(),
        Focus::Centre => state.filtered.len(),
        Focus::Right => pack.required_by.len(),
        Focus::Provides => pack.provides.len(),
    };
    let tstate = match state.focus {
        Focus::Left => &mut state.left_table_state,
        Focus::Centre => &mut state.centre_table_state,
        Focus::Right => &mut state.right_table_state,
        Focus::Provides => &mut state.provides_table_state,
    };

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
                .map(|s| (s + change as usize).min(len - 1)),
        );
    }
}

fn current_pack(state: &AppState) -> Option<&Package> {
    state
        .filtered
        .get(state.centre_table_state.selected().unwrap_or_default())
}
fn get_pack<'a>(state: &'a AppState, name: &str) -> Option<&'a Package> {
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

    let sel_style = if state.focus == Focus::Centre {
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
                .enumerate()
                .map(|(i, c)| {
                    let style = if state.sort_by.0 == i + 1 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    Cell::from(c).style(style)
                })
                .collect::<Row>()
                .style(Style::default().underlined().bold()),
        )
        .highlight_style(sel_style);
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
        .title_bottom(if state.selected.is_empty() {
            "".to_string()
        } else {
            format!("{} selected", state.selected.len())
        })
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
    let pack = current_pack(state);
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
    let pack = current_pack(state);
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

fn draw_provides(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    let pack = current_pack(state);
    if pack.is_none() {
        return Ok(());
    }
    let pack = pack.unwrap();
    let count = pack.provides.len();
    let rows: Vec<Row> = pack
        .provides
        .iter()
        .map(|pro| Row::new(vec![Cell::from(pro.clone())]))
        .collect();
    let style = if state.focus == Focus::Provides {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default()
    };
    state.message = "Prov".into();

    let table = Table::new(rows, [Constraint::Min(0)]).highlight_style(style);
    let block = Block::default()
        .title(format!("Provides {count}"))
        .borders(Borders::all());
    let table = table.block(block);
    f.render_stateful_widget(&table, rect, &mut state.provides_table_state);
    Ok(())
}

fn pacman_exists() -> bool {
    Command::new("pacman").output().is_ok()
}

fn get_packs() -> Result<Vec<Package>, Box<dyn std::error::Error>> {
    let output = Command::new("pacman").arg("-Qil").output()?;
    let output = String::from_utf8(output.stdout)?;
    let mut packs: Vec<Package> = vec![];
    let mut pack = Package::default();

    for line in output.lines() {
        //if listing provides:
        if !pack.name.is_empty() && line.starts_with(&pack.name) {
            if let Some((_, pr)) = line.split_once(" ") {
                pack.provides.push(pr.to_string());
            }
            continue;
        }

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
