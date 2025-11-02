use widgets::table::TableFocus;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Collecting packages...");
    if !pacman_exists() {
        println!("pacman is not installed");
        std::process::exit(1);
    }

    let mut state = AppState::new();

    let res = refresh_packages(&mut state);
    if let Err(e) = res {
        eprintln!("Error getting package list: {e}");
        std::process::exit(1);
    }

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
            let _start = Instant::now();

            use Constraint::{Length, Min};
            let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
            let [header_area, inner_area, footer_area] = vertical.areas(f.area());

            draw_tabs(&state, f, header_area);
            match state.tab {
                Tab::Installed => {
                    state.only_installed = true;
                    draw_packages(&mut state, f, inner_area);
                }
                Tab::Packages => {
                    state.only_installed = false;
                    draw_packages(&mut state, f, inner_area)
                }
                Tab::Updates => draw_updates(&mut state, f, inner_area),
            }
            draw_status(&mut state, f, footer_area).unwrap();

            //overlays
            draw_help(&mut state, f).unwrap();
            draw_command(&mut state, f).unwrap();

            //draw time taken in ms on bottom right corner
            // draw_time_taken(f, _start);
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
                    state.message = format!("Running command...");
                    let _ = goto_screen(false, terminal);
                    let res = run_command(&mut state, c);
                    let _ = goto_screen(true, terminal);
                    if let Err(e) = res {
                        state.message = e.to_string();
                    } else {
                        state.message = format!("Command completed.");
                    }
                }
                EventResult::Queue(_) => unreachable!(),
            }
        }
    }
}

#[allow(dead_code)]
fn draw_time_taken(f: &mut Frame<'_>, _start: Instant) {
    let duration = Instant::now().duration_since(_start);
    let time_text = format!("{} ms", duration.as_millis());
    let time_paragraph = Paragraph::new(time_text)
        .style(Style::default().fg(Color::LightGreen))
        .alignment(Alignment::Right);
    let time_area = Rect {
        x: f.area().width.saturating_sub(11),
        y: f.area().height.saturating_sub(1),
        width: 10,
        height: 1,
    };
    f.render_widget(time_paragraph, time_area);
}

fn draw_tabs(state: &AppState, f: &mut Frame<'_>, header_area: Rect) {
    Tabs::new(Tab::values())
        .highlight_style((Color::Black, Color::Yellow))
        .select(&state.tab)
        .block(Block::bordered())
        .render(header_area, f.buffer_mut());
}

fn draw_updates(state: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    state.update_widget.clone().render(area, f.buffer_mut());
}

fn draw_packages(state: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let info = if state.show_info { 5 } else { 0 };
    let pr = if state.show_providing && state.only_installed {
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

fn handle_event(state: &mut AppState) -> Result<EventResult, Box<dyn Error>> {
    if let Event::Key(key) = event::read()? {
        //priority is ctrl+c
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                state.selected.clear();
                return Ok(EventResult::Quit);
            }
            _ => {}
        }

        //next we handle based on focus
        let handled = match state.focus() {
            Focus::Left => state.left_table.handle_key_event(&key),
            Focus::Right => state.right_table.handle_key_event(&key),
            Focus::Provides => state.provides_table.handle_key_event(&key),

            Focus::Centre => {
                let handled = if state.only_installed {
                    state.installed_table.handle_key_event(&key)
                } else {
                    state.packages_table.handle_key_event(&key)
                };
                if !handled {
                    match key.code {
                        KeyCode::Char('e') => {
                            state.only_expl = !state.only_expl;
                        }
                        KeyCode::Char('o') => {
                            state.only_orphans = !state.only_orphans;
                        }
                        KeyCode::Char('f') => {
                            state.only_foreign = !state.only_foreign;
                        }
                        KeyCode::Char('c') => {
                            state.change_focus(Focus::Command);
                        }
                        KeyCode::Char('P') => cycle_focus_vert(state),
                        KeyCode::Char('p') => state.show_providing = !state.show_providing,
                        _ => {}
                    }
                }
                update_tables(state); //always update tables, as change filter means new pacakges; and change current package means new deps
                update_selection(state);

                handled
            }
            Focus::Updates => {
                if let Some(state) = state.update_widget.handle_key_event(&key) {
                    return Ok(state);
                }
                false
            }

            Focus::Command => {
                state.restore_focus();
                match key.code {
                    KeyCode::Char('r') => {
                        return Ok(EventResult::Command(EventCommand::RemoveSelected));
                    }
                    KeyCode::Char('q') => {
                        return Ok(EventResult::Command(EventCommand::QuerySelected));
                    }

                    _ => {}
                }

                return Ok(EventResult::None);
            }
            Focus::Help => match key.code {
                KeyCode::Char('?') | KeyCode::Esc => {
                    state.restore_focus();
                    return Ok(EventResult::None);
                }
                _ => return Ok(EventResult::None), //no other actions allowed
            },
        };

        //if handled by views, we dont process any global events
        if handled {
            return Ok(EventResult::None);
        }

        //final global key handling
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('?') => state.change_focus(Focus::Help),
                KeyCode::Char('q') => return Ok(EventResult::Quit),
                KeyCode::Tab => {
                    state.tab.cycle_next();
                    match state.tab {
                        Tab::Installed => {
                            state.only_installed = true;
                            state.change_focus(Focus::Centre);
                        }
                        Tab::Packages => {
                            state.only_installed = false;
                            state.change_focus(Focus::Centre);
                        }
                        Tab::Updates => {
                            state.change_focus(Focus::Updates);
                        }
                    };

                    update_dependency_tables(state);
                    return Ok(EventResult::None);
                }
                KeyCode::Char('s') => {
                    return Ok(EventResult::Command(EventCommand::SyncDatabase));
                }

                KeyCode::Char('i') => state.show_info = !state.show_info,
                KeyCode::Left | KeyCode::Char('h') => cycle_focus_horiz(state, -1),
                KeyCode::Right | KeyCode::Char('l') => cycle_focus_horiz(state, 1),
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
    Ok(EventResult::None)
}

fn update_selection(state: &mut AppState) {
    if state.focus() == Focus::Centre {
        let table = if state.only_installed {
            &state.installed_table
        } else {
            &state.packages_table
        };

        state.selected = table
            .get_selected()
            .iter()
            .filter_map(|i| i.cells.get(0))
            .cloned()
            .collect::<Vec<String>>();
    }
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
    let (comm, mut args, needs_package_list) = match command {
        EventCommand::RemoveSelected => ("pacman", vec!["-R"], true),
        EventCommand::UpdateSelected => ("pacman", vec!["-S"], true),
        EventCommand::QuerySelected => ("pacman", vec!["-Qi"], true),
        EventCommand::SyncDatabase => ("pacman", vec!["-Sy"], false),
    };
    if needs_package_list && state.selected.is_empty() {
        return Err(String::from("No packages selected").into());
    }

    args.extend(state.selected.iter().map(|a| a.as_str()));

    std::io::stdout()
        .write_all(format!("\nRunning command: {} {}\n", comm, args.join(" ")).as_bytes())?;
    //try run command as is
    let res = Command::new(comm).args(&args).status()?;

    if !res.success() {
        std::io::stdout().write_all("running sudo\n".as_bytes())?;
        //run as sudo
        args.insert(0, comm);
        args.insert(0, "-S"); //for sudo
        let res = Command::new("sudo").args(&args).status()?;
        if !res.success() {
            std::io::stdout().write_all("Failed to run command".as_bytes())?;
        }
    }
    std::io::stdout().write_all("\nPress enter to continue...".as_bytes())?;
    std::io::stdout().flush()?;
    crossterm::event::read()?;

    refresh_packages(state)?;

    Ok(())
}

fn refresh_packages(state: &mut AppState) -> Result<(), AppError> {
    //run these in parallel
    let jh1 = std::thread::spawn(|| get_installed_packages());
    let jh2 = std::thread::spawn(|| get_all_packages());
    let jh3 = std::thread::spawn(|| get_updates());

    //now join threads
    let installed = jh1.join().expect("Thread error")?;
    let all = jh2.join().expect("Thread error")?;
    let updates = jh3.join().expect("Thread error")?;

    state.packages = combine_packages(installed, all, updates);
    state.update_widget.set_data(
        &state
            .packages
            .iter()
            .filter(|a| a.new_version.is_some())
            .map(|p| PackageUpdate {
                name: p.name.clone(),
                current_version: p.version.clone(),
                new_version: p.new_version.clone().unwrap(),
                change_type: p.change_type.clone().unwrap(),
            })
            .collect::<Vec<_>>(),
    );

    update_tables(state);
    Ok(())
}

fn combine_packages(
    installed: Vec<Package>,
    all: Vec<Package>,
    updates: Vec<PackageUpdate>,
) -> Vec<Package> {
    //start with installed, this may include those not in repo
    let installed_names = installed
        .iter()
        .map(|p| p.name.clone())
        .collect::<HashSet<_>>();
    let mut combined = installed;
    //we now add all local packages not installed
    for pack in all.iter() {
        if !installed_names.contains(&pack.name) {
            combined.push(pack.clone());
        }
    }

    //we add update info
    for pack in updates.iter() {
        if let Some(p) = combined.iter_mut().find(|p| p.name == pack.name) {
            p.new_version = Some(pack.new_version.clone());
            p.change_type = Some(pack.change_type.clone());
        }
    }

    combined
}

fn update_tables(state: &mut AppState) {
    //installed
    let packs: Vec<_> = state
        .packages
        .iter()
        .filter(|p| p.installed.is_some())
        .filter(|p| !state.only_expl || p.reason == Reason::Explicit) //only show explicit packages
        .filter(|p| !state.only_foreign || !p.validated) //only show foreign packages
        .filter(|p| {
            !state.only_orphans
                || p.required_by.is_empty()
                    && p.optional_for.is_empty()
                    && p.reason == Reason::Dependency
        }) //only show orphans
        .cloned()
        .collect();
    let rows: Vec<TableRow> = packs
        .iter()
        .map(|pack| {
            let highlighted = if pack.reason == Reason::Explicit {
                Some(Color::Green)
            } else {
                None
            };
            TableRow {
                cells: vec![
                    pack.name.clone(),
                    format!("{:?}", pack.reason),
                    pack.required_by.len().to_string(),
                    format!("{}", if pack.validated { "" } else { "X" }),
                    pack.installed.clone().expect("filtered installed only"),
                ],
                highlight: highlighted,
            }
        })
        .collect();

    state.installed_table.set_data(rows);

    let count = state.installed_table.rows().len();
    let local = state
        .installed_table
        .rows()
        .iter()
        .filter(|p| !p.cells[3].is_empty())
        .count();
    let foreign = count - local;
    let extra = if foreign > 0 {
        format!(" ({local} pacman, {foreign} foreign)")
    } else {
        "".to_string()
    };

    let mut filters = vec![];
    if state.only_expl {
        filters.push("Explicit")
    }
    if state.only_foreign {
        filters.push("Foreign");
    }
    if state.only_orphans {
        filters.push("Orphans");
    }
    let filters = if filters.is_empty() {
        String::new()
    } else {
        format!("Filters: {}", filters.join(", "))
    };

    let title = format!("Installed {count}{extra} {filters}");
    state.installed_table.set_title(&title);

    //all packages
    let rows = state
        .packages
        .iter()
        .map(|pack| {
            let highlighted = if pack.installed.is_some() {
                Some(Color::Green)
            } else {
                None
            };
            TableRow {
                cells: vec![
                    pack.name.clone(),
                    pack.installed.clone().unwrap_or_default(),
                ],
                highlight: highlighted,
            }
        })
        .collect();

    state.packages_table.set_data(rows);

    let count = state.packages_table.rows().len();

    let installed = state
        .packages_table
        .rows()
        .iter()
        .filter(|p| !p.cells[1].is_empty())
        .count();

    state
        .packages_table
        .set_title(&format!("Packages {count} ({installed} installed)"));

    update_dependency_tables(state);
}
fn update_dependency_tables(state: &mut AppState) {
    //dependents
    if let Some(pack) = current_pack(state) {
        let count = pack.required_by.len();
        let rows: Vec<TableRow> = pack
            .required_by
            .iter()
            .map(|dep| TableRow {
                cells: vec![dep.clone()],
                highlight: None,
            })
            .collect();
        state.right_table.set_data(rows);
        state.right_table.set_title(&format!("Required by {count}"));
    }
    //dependencies
    if let Some(pack) = current_pack(state) {
        let rows: Vec<TableRow> = pack
            .dependencies
            .iter()
            .map(|dep| {
                let col: Option<Color> = match get_pack(state, dep) {
                    Some(p) if p.reason == Reason::Explicit => Some(Color::Green),
                    Some(_) => None,
                    None => Some(Color::Red),
                };

                TableRow {
                    cells: vec![dep.clone()],
                    highlight: col,
                }
            })
            .collect();
        let count = pack.dependencies.len();

        let title = format!("Depends on {count}");
        state.left_table.set_title(&title);
        state.left_table.set_data(rows);
    }
}

fn handle_enter(state: &mut AppState) {
    let pack = current_pack(state);
    if pack.is_none() {
        return;
    }
    let name = match state.focus() {
        Focus::Left => state.left_table.current().map(|a| a.cells[0].clone()),
        Focus::Right => state.right_table.current().map(|a| a.cells[0].clone()),
        _ => return,
    }
    .unwrap_or_default();

    let prevpack = current_pack(state).cloned();
    //clear current filters from centre table
    if state.only_installed {
        state.installed_table.clear_selection();
        state.installed_table.clear_search();
    } else {
        state.packages_table.clear_selection();
        state.packages_table.clear_search();
    }

    if let Some(pack) = get_pack(state, &name).cloned() {
        //undo any filters
        state.only_expl = false;

        if let Some(prevpack) = prevpack {
            let prev = prevpack.name.clone();
            goto_package(state, &pack.name.clone());
            state.prev.push(prev);
        }
    }
    update_tables(state);
}

fn goto_package(state: &mut AppState, name: &str) {
    state.change_focus(Focus::Centre);
    let table_rows = if state.only_installed {
        state.installed_table.rows()
    } else {
        state.packages_table.rows()
    };
    //find new index in table rows
    let new_index = table_rows
        .iter()
        .position(|p| p.cells[0] == name)
        .unwrap_or_default();

    state.installed_table.select(Some(new_index));
}

fn cycle_focus_vert(state: &mut AppState) {
    state.change_focus(match state.focus() {
        Focus::Provides => Focus::Centre,
        _ => {
            if state.show_providing {
                Focus::Provides
            } else {
                Focus::Centre
            }
        }
    });
}

fn cycle_focus_horiz(state: &mut AppState, arg: i32) {
    let pack = current_pack(state);
    if pack.is_none() {
        return;
    }
    let pack = pack.unwrap();
    let left_count = pack.dependencies.len();
    let right_count = pack.required_by.len();

    state.change_focus(if arg > 0 {
        match state.focus() {
            Focus::Left => Focus::Centre,
            Focus::Centre if right_count > 0 => Focus::Right,
            _ => state.focus(), //else keep
        }
    } else {
        match state.focus() {
            Focus::Centre if left_count > 0 => Focus::Left,
            Focus::Right => Focus::Centre,
            _ => state.focus(), //else keep
        }
    });
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
    if state.focus() != Focus::Command {
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
    if state.focus() != Focus::Help {
        return Ok(());
    }

    let mut commands = vec![
        "?: Toggle Help".to_string(),
        "q: Quit".to_string(),
        "s: Sync Database".to_string(),
    ];

    //use previous focus because current focus is help
    let extra = match state.focus_previous() {
        Focus::Left => vec![],
        Focus::Centre => vec![],
        Focus::Right => vec![],
        Focus::Provides => vec![],
        Focus::Updates => state.update_widget.command_descriptions(),
        Focus::Command => vec![],
        Focus::Help => vec![],
    };
    let formatted = extra
        .into_iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>();
    commands.extend(formatted);

    let size = f.area();

    // Calculate the block size (1/3 of the screen size)
    let block_width = (size.width / 3).max(50);
    let block_height = (size.height / 3).min(22).min(commands.len() as u16 + 2);

    // Calculate the block position (centered)
    let block_x = (size.width - block_width) / 2;
    let block_y = (size.height - block_height) / 2;

    // Create a centered block
    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Blue).fg(Color::Black));

    // Create a paragraph to display inside the block
    let paragraph = Paragraph::new(commands.into_iter().map(|s| s.into()).collect::<Vec<_>>())
        /*
            "i: Info".into(),
            "Tab: Switch tab".into(),
            "Left/Right/h/l: Switch column".into(),
            "Up/Down/k/j: Navigate".into(),
            "Esc: Clear filters and selection".into(),
            "e: Explicitly installed only".into(),
            "f: Foreign packages only".into(),
            "o: Orphans only".into(),
            "/: Search".into(),
            "[1-9]: Sort column".into(),
            "Enter (on outer columns): Goto Package".into(),
            "Backspace: Previous package".into(),
            "Tab: Next tab".into(),
            "Space: Select/Deselect".into(),
            "c: Run command on selection".into(),
            "p: Provides".into(),
            "Shift+p: Switch focus to/from provides tab".into(),
        ])*/
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

    let prev;
    if !state.prev.is_empty() {
        prev = format!("BSP: Back to '{}'", state.prev.last().unwrap());
        text.push(&prev);
    }

    if !state.message.is_empty() {
        text.push(&state.message);
    }

    let para = Paragraph::new(text.join(" ")).style(Style::default().fg(Color::Yellow));
    f.render_widget(&para, rect);
    Ok(())
}

fn current_pack(state: &AppState) -> Option<&Package> {
    if state.only_installed {
        get_package_from_table(&state.installed_table, &state.packages)
    } else {
        get_package_from_table(&state.packages_table, &state.packages)
    }
}

fn get_package_from_table<'a>(
    table: &'a TableWidget,
    packages: &'a [Package],
) -> Option<&'a Package> {
    if let Some(curr) = table.current() {
        packages.iter().find(|p| p.name == curr.cells[0])
    } else {
        None
    }
}
fn get_pack<'a>(state: &'a AppState, name: &str) -> Option<&'a Package> {
    state.packages.iter().find(|p| p.name == name)
}

///draws list, plus package info
fn draw_centre(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    //Packages
    let mut table = if state.only_installed {
        state.installed_table.clone()
    } else {
        state.packages_table.clone()
    };
    table.focus(if state.focus() == Focus::Centre {
        TableFocus::Focused
    } else {
        TableFocus::UnfocusedDimmed
    });
    f.render_widget(table, rect);
    Ok(())
}

fn draw_dependencies(
    state: &mut AppState,
    f: &mut Frame,
    rect: Rect,
) -> Result<(), Box<dyn Error>> {
    state.left_table.focus(if state.focus() == Focus::Left {
        TableFocus::Focused
    } else {
        TableFocus::Unfocused
    });
    state.left_table.clone().render(rect, f.buffer_mut());
    Ok(())
}
fn draw_dependents(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    state.right_table.focus(if state.focus() == Focus::Right {
        TableFocus::Focused
    } else {
        TableFocus::Unfocused
    });
    state.right_table.clone().render(rect, f.buffer_mut());
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
    let Some(pack) = current_pack(state) else {
        return Ok(());
    };
    let rows: Vec<TableRow> = pack
        .provides
        .iter()
        .map(|pro| TableRow {
            cells: vec![pro.clone()],
            highlight: None,
        })
        .collect();

    let count = pack.provides.len();
    let title = format!("Provides {count}");
    state.provides_table.set_title(&title);
    state.provides_table.set_data(rows);
    state.provides_table.clone().render(rect, f.buffer_mut());
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
        if let Some(pack) = state.packages.iter_mut().find(|p| p.name == name) {
            pack.provides = provides.clone();
        }
    }
}

fn pacman_exists() -> bool {
    Command::new("pacman").output().is_ok()
}

fn get_packages_command(command: &str) -> Result<Vec<Package>, AppError> {
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
            "Install Date" => pack.installed = Some(to_date(value)),
            "Description" => pack.description = value.to_string(),
            "Validated By" => pack.validated = value == "Signature",
            _ => {}
        }
    }
    packs.push(pack);

    Ok(packs)
}

fn get_all_packages() -> Result<Vec<Package>, AppError> {
    let packs = get_packages_command("-Si")?;
    Ok(packs)
}

fn get_installed_packages() -> Result<Vec<Package>, AppError> {
    get_packages_command("-Qi")
}

fn get_updates() -> Result<Vec<PackageUpdate>, AppError> {
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
            let current = Version::from(parts[1]);
            let new = Version::from(parts[2]);
            updates.push(PackageUpdate {
                name: parts[0].to_string(),
                current_version: parts[1].to_string(),
                new_version: parts[2].to_string(),
                change_type: current.change_type(&new),
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
}

pub mod error;
pub mod structs;
pub mod utils;
pub mod version;
pub mod widgets;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Offset, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, Tabs, Widget},
};
use std::{collections::HashSet, error::Error, io::Write, process::Command, time::Instant};
use structs::{AppState, EventResult, Focus, Package, Reason};

use crate::{
    error::AppError,
    structs::{EventCommand, PackageUpdate, Tab},
    version::Version,
    widgets::{
        Commands,
        table::{TableRow, TableWidget},
    },
};
