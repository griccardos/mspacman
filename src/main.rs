pub mod error;
pub mod pman;
pub mod structs;
pub mod utils;
pub mod version;
pub mod widgets;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, Tabs, Widget},
};
use std::{error::Error, time::Instant};

use crate::{
    error::AppError,
    pman::{pacman_exists, refresh_packages_and_update_tables, run_command},
    structs::{
        appstate::AppState,
        event::{EventCommand, EventResult},
        package::Package,
        tab::Tab,
    },
    widgets::{Commands, CurrentPackage},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Collecting packages...");
    if !pacman_exists() {
        println!("pacman is not installed");
        std::process::exit(1);
    }

    let mut state = AppState::default();

    let res = refresh_packages_and_update_tables(&mut state);
    if let Err(e) = res {
        eprintln!("Error getting package list: {e}");
        std::process::exit(1);
    }

    let mut terminal = ratatui::init();
    terminal.clear()?;
    let appresult = run(&mut terminal, state);
    ratatui::restore();

    if let Err(e) = appresult {
        eprintln!("{e}");
        std::process::exit(1);
    }
    Ok(())
}

fn run(terminal: &mut DefaultTerminal, mut state: AppState) -> Result<(), AppError> {
    loop {
        terminal.draw(|f| {
            let _start = Instant::now();
            let info = if state.show_info { 5 } else { 0 };

            use Constraint::{Length, Min};
            let vertical = Layout::vertical([Length(3), Min(0), Length(info), Length(1)]);
            let [header_area, inner_area, info_area, footer_area] = vertical.areas(f.area());

            draw_tabs(&state, f, header_area);

            match state.tab {
                Tab::Installed | Tab::Packages => draw_packages(&mut state, f, inner_area),
                Tab::Updates => draw_updates(&mut state, f, inner_area),
            }
            draw_info(&mut state, f, info_area).unwrap();
            draw_status(&mut state, f, footer_area).unwrap();
            draw_help(&mut state, f).unwrap();

            //draw time taken in ms on bottom right corner
            //draw_time_taken(f, _start);
        })?;

        let ev = handle_event(&mut state)?;

        match ev {
            EventResult::None => {}
            EventResult::Quit => return Ok(()),
            EventResult::Command(c) => {
                state.message = "Running command...".to_string();
                let _ = goto_screen(false, terminal);
                let res = run_command(&mut state, c);
                let _ = goto_screen(true, terminal);
                if let Err(e) = res {
                    state.message = e.to_string();
                } else {
                    state.message = "Command completed.".to_string();
                }
            }
            EventResult::NeedsUpdate => {
                refresh_packages_and_update_tables(&mut state)?;
            } //EventResult::Queue(_) => unreachable!(),
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
    if state.tab == Tab::Installed {
        state.installed_widget.clone().render(area, f.buffer_mut());
    } else if state.tab == Tab::Packages {
        state.packages_widget.clone().render(area, f.buffer_mut());
    }
}

fn handle_event(state: &mut AppState) -> Result<EventResult, AppError> {
    if let Event::Key(key) = event::read()? {
        //priority is ctrl+c
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(EventResult::Quit);
            }
            _ => {}
        }

        //if showing help
        if state.show_help {
            match key.code {
                KeyCode::Char('?') | KeyCode::Esc => {
                    state.show_help = false;
                }
                _ => {}
            }
            //no other actions allowed
            return Ok(EventResult::None);
        }

        //next we handle based on focus
        let res = match state.tab {
            Tab::Installed => state.installed_widget.handle_key_event(&key),
            Tab::Packages => state.packages_widget.handle_key_event(&key),
            Tab::Updates => state.update_widget.handle_key_event(&key),
        };
        if let Some(res) = res {
            return Ok(res);
        }

        //final global key handling
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('?') => state.show_help = true,
                KeyCode::Char('q') => return Ok(EventResult::Quit),
                KeyCode::Tab => {
                    state.tab.cycle_next();
                    return Ok(EventResult::None);
                }
                KeyCode::Char('s') => {
                    return Ok(EventResult::Command(EventCommand::SyncDatabase));
                }

                KeyCode::Char('i') => state.show_info = !state.show_info,

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

fn update_tables(state: &mut AppState) {
    //installed
    let packs: Vec<_> = state
        .packages
        .iter()
        .filter(|p| p.installed.is_some())
        .cloned()
        .collect();
    state.installed_widget.set_data(packs);

    //all packages
    state.packages_widget.set_data(&state.packages);

    //updates
    state.update_widget.set_data(
        &state
            .packages
            .iter()
            .filter(|a| a.new_version.is_some())
            .cloned()
            .collect::<Vec<_>>(),
    );
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
fn current_pack(state: &AppState) -> Option<&Package> {
    match state.tab {
        Tab::Installed => state.installed_widget.current_package(),
        Tab::Packages => state.packages_widget.current_package(),
        Tab::Updates => state.update_widget.current_package(),
    }
}

fn draw_help(state: &mut AppState, f: &mut Frame) -> Result<(), Box<dyn Error>> {
    if !state.show_help {
        return Ok(());
    }

    let mut commands = vec![
        "?: Toggle Help".to_string(),
        "q: Quit".to_string(),
        "s: Sync Database".to_string(),
        "/: Search".to_string(),
        "i: Toggle Info Panel".to_string(),
        "Ctrl+a: Toggle select all".to_string(),
        "Esc: Clear Filter".to_string(),
        "1-9: Sort column".to_string(),
        "".to_string(),
    ];

    //use previous focus because current focus is help
    let extra = match state.tab {
        Tab::Updates => state.update_widget.command_descriptions(),
        Tab::Installed => state.installed_widget.command_descriptions(),
        Tab::Packages => state.packages_widget.command_descriptions(),
    };
    let formatted = extra
        .into_iter()
        .map(|(k, v, _)| format!("{}: {}", k, v))
        .collect::<Vec<_>>();
    commands.extend(formatted);

    let size = f.area();

    // Calculate the block size (1/3 of the screen size)
    let block_width = (size.width / 3).max(50);
    let block_height = commands.len() as u16 + 2;

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
        .block(block)
        .alignment(Alignment::Left);
    let rect = Rect::new(block_x, block_y, block_width, block_height);
    // Render the block
    f.render_widget(Clear, rect);
    f.render_widget(paragraph, rect);

    Ok(())
}
fn draw_status(state: &mut AppState, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
    let mut text = vec![" ?:Help", "Tab:Change view", "/:Search", "s:Sync"];

    let extra = match state.tab {
        Tab::Updates => state.update_widget.command_descriptions(),
        Tab::Installed => state.installed_widget.command_descriptions(),
        Tab::Packages => state.packages_widget.command_descriptions(),
    };
    let formatted = extra
        .into_iter()
        .filter(|(_, _, v)| !v.is_empty())
        .map(|(k, _, v)| format!("{}:{}", k, v))
        .collect::<Vec<_>>();

    text.extend(formatted.iter().map(|s| s.as_str()));

    let layout =
        Layout::horizontal([Constraint::Percentage(80), Constraint::Percentage(20)]).split(rect);

    let info = Paragraph::new(text.join("  ")).style(Style::default().fg(Color::Yellow));
    f.render_widget(&info, layout[0]);
    Text::raw(&state.message)
        .style(Style::default().fg(Color::Red))
        .render(layout[1], f.buffer_mut());

    Ok(())
}
