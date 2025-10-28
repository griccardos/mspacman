fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !pacman_exists() {
        println!("pacman is not installed");
        std::process::exit(1);
    }

    let installed = get_installed_packages()?;
    let all_packs = get_all_packages(&installed)?;
    let updates = get_updates()?;

    let state = AppState {
        filtered: installed.clone(),
        packages_installed: installed,
        packages_all: all_packs,
        packages_updates: updates,
        show_info: true,
        sort_by: (1, Sort::Asc),
        hide_columns: HashMap::from_iter([(2, false), (3, false), (4, false), (5, false)]),
        search_input: get_textarea("Search..."),
        left_table_state: TableState::default().with_selected(Some(0)),
        centre_table_state: TableState::default().with_selected(Some(0)),
        right_table_state: TableState::default().with_selected(Some(0)),
        ..Default::default()
    };

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
    loop {
        terminal.draw(|f| {
            //draw_installed(&mut state, &mut search_input, f);
            use Constraint::{Length, Min};
            let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
            let [header_area, inner_area, footer_area] = vertical.areas(f.area());

            draw_tabs(&state, f, header_area);
            match state.tab {
                Tab::Installed => {
                    state.only_installed = true;
                    update_filter(&mut state);
                    draw_packages(&mut state, f, inner_area);
                }
                Tab::Packages => {
                    state.only_installed = false;
                    update_filter(&mut state);
                    draw_packages(&mut state, f, inner_area)
                }
                Tab::Updates => draw_updates(&mut state, f, inner_area),
            }
            draw_status(&mut state, f, footer_area).unwrap();

            //overlays
            draw_help(&mut state, f).unwrap();
            draw_command(&mut state, f).unwrap();
        })?;

        let ev = handle_event(&mut state)?;
        let evs = match ev {
            EventResult::Queue(event_results) => event_results,
            _ => vec![ev],
        };
        for ev in evs {
            match ev {
                EventResult::None => {}
                EventResult::Quit => return Ok(state.selected),
                EventResult::Select(names) => {
                    state.selected = names;
                }
                EventResult::Command(c) => {
                    let _ = goto_screen(false, terminal);
                    let res = run_command(&mut state, c);
                    let _ = goto_screen(true, terminal);
                    if let Err(e) = res {
                        state.message = e.to_string();
                    }
                }
                EventResult::Queue(_) => unreachable!(),
            }
        }
    }
}

fn draw_tabs(state: &AppState, f: &mut Frame<'_>, header_area: Rect) {
    Tabs::new(Tab::values())
        .highlight_style((Color::Black, Color::Yellow))
        .select(&state.tab)
        .render(header_area, f.buffer_mut());
}

fn draw_updates(state: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    state.update_widget.set_data(&state.packages_updates);
    state.update_widget.clone().render(area, f.buffer_mut());
}

fn draw_packages(state: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let info = if state.show_info { 6 } else { 0 };
    let pr = if state.show_providing {
        ((area.height as f32 * 0.5) as u16).max(5)
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
        .split(area);
    let vert_splits = if state.only_installed {
        [20, 60, 20]
    } else {
        [20, 80, 0]
    };
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(Constraint::from_percentages(vert_splits))
        .split(top_bottom[0]);

    draw_dependencies(state, f, body[0]).unwrap();
    draw_centre(state, f, body[1]).unwrap();
    if state.only_installed {
        draw_dependents(state, f, body[2]).unwrap();
    }
    draw_info(state, f, top_bottom[1]).unwrap();
    draw_provides(state, f, top_bottom[2]).unwrap();
}

fn get_textarea(label: &str) -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_placeholder_text(label);
    textarea.set_style(Style::default().bg(Color::Gray).fg(Color::Black));
    textarea.set_placeholder_style(Style::default().bg(Color::Gray).fg(Color::DarkGray));
    textarea
}

fn clear_search(state: &mut AppState) {
    state.search_input.select_all();
    state.search_input.cut();
    state.filter = String::new();
    update_filter(state);
}
fn handle_event(state: &mut AppState) -> Result<EventResult, Box<dyn Error>> {
    if let Event::Key(key) = event::read()? {
        //global key handling
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('?') => {
                    state.show_help = !state.show_help;
                    if state.show_help {
                        state.focus_previous = state.focus;
                        state.focus = Focus::Help;
                    } else {
                        state.focus = state.focus_previous;
                    }

                    return Ok(EventResult::None);
                }
                KeyCode::Char('q') => return Ok(EventResult::Quit),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.selected.clear();
                    return Ok(EventResult::Quit);
                }
                KeyCode::Tab | KeyCode::Char('l') => {
                    state.tab.cycle_next();
                    state.focus = match state.tab {
                        Tab::Installed => Focus::Centre,
                        Tab::Packages => Focus::Centre,
                        Tab::Updates => Focus::Updates,
                    };
                    return Ok(EventResult::None);
                }
                KeyCode::Char('h') => {
                    state.tab.cycle_prev();
                    state.focus = match state.tab {
                        Tab::Installed => Focus::Centre,
                        Tab::Packages => Focus::Centre,
                        Tab::Updates => Focus::Updates,
                    };
                    return Ok(EventResult::None);
                }
                _ => {}
            }
        }

        match state.focus {
            Focus::Left => {}
            Focus::Centre => {}
            Focus::Right => {}
            Focus::Provides => {}
            Focus::Updates => {
                return state.update_widget.handle_key_event(&key);
            }
            Focus::Help => {
                //cannot do other ops in help
                return Ok(EventResult::None);
            }
        }
        //if searching, we handle input and return
        if state.searching {
            match key.code {
                KeyCode::Esc => {
                    state.searching = false;
                    clear_search(state);
                    return Ok(EventResult::None);
                }
                KeyCode::Enter => {
                    state.searching = false;
                    return Ok(EventResult::None);
                }
                KeyCode::Up | KeyCode::Down => {
                    state.searching = false;
                }
                _ => {}
            }
            state.search_input.input(key);
            state.filter = state.search_input.lines().join(" ");
            update_filter(state);

            return Ok(EventResult::None);
        }
        //if searching, we handle input and return
        if state.show_command {
            state.show_command = false;
            match key.code {
                KeyCode::Char('r') => {
                    return Ok(EventResult::Command(EventCommand::RemoveSelected));
                }
                KeyCode::Char('q') => {
                    return Ok(EventResult::Command(EventCommand::QuerySelected));
                }
                KeyCode::Char('s') => {
                    return Ok(EventResult::Command(EventCommand::SyncUpdateSelected));
                }
                _ => {}
            }

            return Ok(EventResult::None);
        }

        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Esc => {
                    state.only_expl = false;
                    state.only_foreign = false;
                    state.only_orphans = false;
                    state.show_help = false;
                    state.show_command = false;
                    state.selected.clear();
                    clear_search(state);
                    update_filter(state);
                }

                KeyCode::Char('c') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.show_command = true;
                }
                KeyCode::Char(val) if ('1'..='5').contains(&val) => {
                    let val = val.to_digit(10).unwrap() as usize;
                    if key.modifiers.contains(KeyModifiers::ALT) {
                        hide_column(state, val)
                    } else {
                        set_sort(state, val);
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
                KeyCode::Char('P') => cycle_focus_vert(state),
                KeyCode::Char('p') => state.show_providing = !state.show_providing,
                KeyCode::Char('/') => state.searching = true,
                KeyCode::Down | KeyCode::Char('j') => safe_move(state, 1),
                KeyCode::Up | KeyCode::Char('k') => safe_move(state, -1),
                KeyCode::PageDown => safe_move(state, 10),
                KeyCode::PageUp => safe_move(state, -10),
                KeyCode::Home => safe_move(state, -isize::MAX),
                KeyCode::End => safe_move(state, isize::MAX),
                KeyCode::Left => cycle_focus_horiz(state, -1),
                KeyCode::Right => cycle_focus_horiz(state, 1),
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
    Ok(EventResult::None)
}

fn goto_screen(alternate: bool, terminal: &mut DefaultTerminal) -> Result<(), Box<dyn Error>> {
    use crossterm::ExecutableCommand;
    use crossterm::terminal::EnterAlternateScreen;
    use crossterm::terminal::LeaveAlternateScreen;
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
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

fn run_command(state: &mut AppState, command: EventCommand) -> Result<(), Box<dyn Error>> {
    if state.selected.is_empty() {
        return Ok(());
    }
    let comm_arg = match command {
        EventCommand::RemoveSelected => Some(("pacman", vec!["-R"])),
        EventCommand::SyncUpdateSelected => Some(("pacman", vec!["-S"])),
        EventCommand::QuerySelected => Some(("pacman", vec!["-Qi"])),
    };

    if let Some((comm, mut args)) = comm_arg {
        args.extend(state.selected.iter().map(|a| a.as_str()));

        std::io::stdout()
            .write_all(format!("\nRunning command: {} {}\n", comm, args.join(" ")).as_bytes())?;
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

    let before = state.packages_installed.len();
    state.packages_installed = get_installed_packages()?;
    state.packages_all = get_all_packages(&state.packages_installed)?;
    state.packages_updates = get_updates()?;

    update_filter(state);
    state.message = format!("{}->{}", before, state.packages_installed.len());
    Ok(())
}

fn hide_column(state: &mut AppState, arg: usize) {
    state.hide_columns.entry(arg).and_modify(|a| *a = !*a);
}

fn set_sort(state: &mut AppState, arg: usize) {
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
}
fn filtered_sort(state: &mut AppState) {
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
    let source = if state.only_installed {
        &state.packages_installed
    } else {
        &state.packages_all
    };
    state.filtered = source
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
    if state.only_installed {
        state
            .selected
            .retain(|p| state.packages_installed.iter().any(|f| f.name == *p));
    } else {
        state
            .selected
            .retain(|p| state.packages_all.iter().any(|f| f.name == *p));
    }
    filtered_sort(state);
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
        Focus::Updates => return,
        Focus::Help => return,
    };

    let prevpack = current_pack(state).cloned();
    if let Some(pack) = get_pack(state, &name).cloned() {
        //undo any filters
        state.searching = false;
        clear_search(state);
        state.only_expl = false;
        state.filtered = if state.only_installed {
            state.packages_installed.clone()
        } else {
            state.packages_all.clone()
        };
        update_filter(state);

        if let Some(prevpack) = prevpack {
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
        Focus::Right => state.right_table_state.select(Some(0)),
        _ => {}
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
        .style(Style::default().bg(Color::Blue).fg(Color::Black));

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
            .map(|s| Line::from(s).style(Style::default().fg(Color::Red))),
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
    let block_height = (size.height / 3).max(22);

    // Calculate the block position (centered)
    let block_x = (size.width - block_width) / 2;
    let block_y = (size.height - block_height) / 2;

    // Create a centered block
    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Blue).fg(Color::Black));

    // Create a paragraph to display inside the block
    let paragraph = Paragraph::new(vec![
        "?: Toggle Help".into(),
        "q: Quit".into(),
        "i: Info".into(),
        "p: Provides".into(),
        "Shift+p: Switch focus to/from provides tab".into(),
        "d: Date".into(),
        "e: Explicitly installed only".into(),
        "f: Foreign packages only".into(),
        "o: Orphans only".into(),
        "/: Search".into(),
        "[2-5]: Sort".into(),
        "Alt+[2-5]: Minimize Column".into(),
        "Enter (on outer columns): Goto Package".into(),
        "Backspace: Go back".into(),
        "Left/Right: Switch column".into(),
        "Tab: Next tab".into(),
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
fn draw_status(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
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

    let prev;
    if !state.prev.is_empty() {
        prev = format!("BSP: Back to '{}'", state.prev.last().unwrap());
        text.push(&prev);
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

    let search_len = if state.searching || !state.search_input.is_empty() {
        state
            .search_input
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
    if state.searching || !state.search_input.is_empty() {
        if state.searching {
            state
                .search_input
                .set_cursor_style(Style::default().bg(Color::White));
        } else {
            state
                .search_input
                .set_cursor_style(Style::default().bg(Color::Gray));
        }
        f.render_widget(&state.search_input, layout[0]);
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
        _ => return,
    };
    let tstate = match state.focus {
        Focus::Left => &mut state.left_table_state,
        Focus::Centre => &mut state.centre_table_state,
        Focus::Right => &mut state.right_table_state,
        Focus::Provides => &mut state.provides_table_state,
        _ => return,
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
    if state.show_providing {
        ensure_has_provides(state);
    }
}

fn current_pack(state: &AppState) -> Option<&Package> {
    state
        .filtered
        .get(state.centre_table_state.selected().unwrap_or_default())
}
fn get_pack<'a>(state: &'a AppState, name: &str) -> Option<&'a Package> {
    if state.only_installed {
        state.packages_installed.iter().find(|p| p.name == name)
    } else {
        state.packages_all.iter().find(|p| p.name == name)
    }
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
            if state.only_installed && pack.reason == Reason::Explicit {
                style = style.fg(Color::Green);
            }
            if !state.only_installed && !pack.installed.is_empty() {
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
    let (head, mut widths) = if state.only_installed {
        (
            vec!["Name", "Reason", "ReqBy", "Foreign", "Installed"],
            vec![
                Constraint::Percentage(50),
                Constraint::Percentage(15),
                Constraint::Min(5),
                Constraint::Min(3),
                Constraint::Length(19),
            ],
        )
    } else {
        (
            vec!["Name", "Reason", "ReqBy", "Foreign", "Installed"],
            vec![
                Constraint::Percentage(50),
                Constraint::Length(0),
                Constraint::Length(0),
                Constraint::Length(0),
                Constraint::Length(19),
            ],
        )
    };

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
        .row_highlight_style(sel_style);
    let count = state.filtered.len();
    let title = if state.only_installed {
        let local = state.filtered.iter().filter(|p| p.validated).count();
        let foreign = count - local;
        let extra = if foreign > 0 {
            format!(" ({local} pacman, {foreign} foreign)")
        } else {
            "".to_string()
        };

        format!("Installed {count}{extra}")
    } else {
        let installed = state
            .filtered
            .iter()
            .filter(|p| !p.installed.is_empty())
            .count();

        format!("Packages {count} ({installed} installed)")
    };
    let block = Block::default()
        .title(title)
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
    let table = Table::new(rows, [Constraint::Min(0)]).row_highlight_style(style);
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

    let table = Table::new(rows, [Constraint::Min(0)]).row_highlight_style(style);
    let block = Block::default()
        .title(format!("Required by {count}"))
        .borders(Borders::all());
    let table = table.block(block);
    f.render_stateful_widget(&table, rect, &mut state.right_table_state);
    Ok(())
}

fn get_provides(package: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::new("pacman").arg("-Ql").arg(package).output()?;
    let output = String::from_utf8(output.stdout)?;
    Ok(output
        .lines()
        .filter_map(|line| {
            if let Some((_, path)) = line.split_once(" ") {
                Some(path.to_string())
            } else {
                None
            }
        })
        .collect())
}

fn draw_provides(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    ensure_has_provides(state);
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

    let table = Table::new(rows, [Constraint::Min(0)]).row_highlight_style(style);
    let block = Block::default()
        .title(format!("Provides {count}"))
        .borders(Borders::all());
    let table = table.block(block);
    f.render_stateful_widget(&table, rect, &mut state.provides_table_state);
    Ok(())
}

fn ensure_has_provides(state: &mut AppState) {
    let mut name = None;
    let mut provides = None;
    let curr = current_pack(state);
    if let Some(pack) = curr {
        if pack.provides.is_empty() {
            if let Ok(prov) = get_provides(&pack.name) {
                name = Some(pack.name.clone());
                provides = Some(prov);
            }
        }
    }
    if let (Some(name), Some(provides)) = (name, provides) {
        if let Some(pack) = state.packages_installed.iter_mut().find(|p| p.name == name) {
            pack.provides = provides.clone();
        }
        if let Some(pack) = state.packages_all.iter_mut().find(|p| p.name == name) {
            pack.provides = provides;
        }
    }
}

fn pacman_exists() -> bool {
    Command::new("pacman").output().is_ok()
}

fn get_packages_command(command: &str) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
    let output = Command::new("pacman")
        .env("LC_TIME", "C")
        .arg(command)
        .output()?;
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

fn get_all_packages(installed: &[Package]) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
    let mut packs = get_packages_command("-Si")?;
    let map = installed
        .iter()
        .map(|p| (p.name.clone(), p.installed.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    //now update installed status
    for pack in packs.iter_mut() {
        if map.contains_key(&pack.name) {
            pack.installed = map[&pack.name].clone();
        }
    }
    Ok(packs)
}
fn get_installed_packages() -> Result<Vec<Package>, Box<dyn std::error::Error>> {
    get_packages_command("-Qi")
}

fn get_updates() -> Result<Vec<PackageUpdate>, Box<dyn std::error::Error>> {
    let output = Command::new("pacman")
        .env("LC_TIME", "C")
        .arg("-Qu")
        .output()?;
    let output = String::from_utf8(output.stdout)?;
    let mut updates: Vec<PackageUpdate> = vec![];
    for line in output.lines() {
        let line = line.replace(" -> ", " ");
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 3 {
            updates.push(PackageUpdate {
                name: parts[0].to_string(),
                current_version: parts[1].to_string(),
                new_version: parts[2].to_string(),
                change_type: change_type(parts[1], parts[2]),
            });
        }
    }

    Ok(updates)
}

fn to_date(value: &str) -> String {
    //get rid of the timezone
    let time = match jiff::fmt::strtime::parse("%a %b %e %H:%M:%S %Y", value) {
        Ok(time) => time,
        Err(e) => panic!("Could not parse '{value}': {e}"),
    };
    time.to_datetime().unwrap().to_string().replace("T", " ")
    //time.to_string("%Y-%m-%d %H:%M:%S").unwrap()
}

pub mod structs;
pub mod version;
pub mod widgets;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Offset, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Tabs, Widget},
};
use std::{collections::HashMap, error::Error, io::Write, process::Command};
use structs::{AppState, EventResult, Focus, Package, Reason, Sort};
use tui_textarea::TextArea;

use crate::{
    structs::{EventCommand, PackageUpdate, Tab},
    version::change_type,
};
