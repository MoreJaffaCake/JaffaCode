mod editor;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui::{
    layout::{Constraint, Direction, Layout, Offset},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::io;
use std::time::Duration;

use editor::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut editors = vec![
        Editor::new(&std::fs::read_to_string(
            option_env!("FILE").unwrap_or("src/editor.rs"),
        )?),
        Editor::new(include_str!("editor/window.rs")),
    ];
    let constraints = std::iter::repeat_n(Constraint::Fill(1), editors.len()).collect::<Vec<_>>();
    let mut active_editor = 0;
    let mut scroll: usize = 1;
    let mut debug = false;
    let mut terminal_size = None;

    loop {
        let draw = {
            let new_size = terminal.size().ok();
            if terminal_size == new_size {
                false
            } else {
                terminal_size = terminal.size().ok();
                true
            }
        };

        if draw {
            terminal.draw(|f| {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(43), Constraint::Fill(2)])
                    .split(f.area());
                let editor_areas = if debug {
                    Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(&constraints)
                        .split(cols[0])
                } else {
                    Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(&constraints)
                        .split(f.area())
                };

                for (i, editor) in editors.iter_mut().enumerate() {
                    let location = editor.location();
                    let editor_comps = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(location.lines.len() as _),
                            Constraint::Percentage(100),
                        ])
                        .split(editor_areas[i]);
                    let block = Block::default().fg(Color::White).bg(Color::Blue);
                    let p = Paragraph::new(
                        location
                            .lines
                            .into_iter()
                            .map(|DisplayLine { slice, indent, .. }| {
                                Line::from(vec![Span::raw(indent), Span::raw(slice)])
                            })
                            .collect::<Vec<_>>(),
                    )
                    .block(block);
                    f.render_widget(p, editor_comps[0]);
                    let mut block = Block::default().fg(Color::Gray);
                    let inner = block.inner(editor_comps[1]);
                    editor.update_pane_size(inner.width, inner.height);
                    if i == active_editor {
                        block = block.fg(Color::White);
                        let (mut x, y) = editor.cursor_position();
                        x += 4;
                        let offset = inner.offset(Offset { x, y }).intersection(inner);
                        if !offset.is_empty() {
                            f.set_cursor_position(offset);
                        }
                    }
                    let p = Paragraph::new(
                        editor
                            .get_display_lines()
                            .map(
                                |DisplayLine {
                                     slice,
                                     indent,
                                     continuation,
                                 }| {
                                    let mut info = Span::raw(format!("{:02}  ", indent.len()));
                                    if continuation {
                                        info = info.fg(Color::Green);
                                    } else if indent.len() > 0 {
                                        info = info.fg(Color::Blue);
                                    } else {
                                        info = info.fg(Color::Gray);
                                    }
                                    Line::from(vec![info, Span::raw(indent), Span::raw(slice)])
                                },
                            )
                            .collect::<Vec<_>>(),
                    )
                    .block(block);
                    f.render_widget(p, editor_comps[1]);
                }

                if debug {
                    let info = format!("{:#?}", editors);
                    let p = Paragraph::new(
                        info.lines()
                            .skip(scroll)
                            .map(|x| Line::from(Span::raw(x)))
                            .collect::<Vec<_>>(),
                    )
                    .wrap(Wrap { trim: false })
                    .block(Block::default().borders(Borders::ALL));
                    f.render_widget(p, cols[1]);
                }
            })?;
        }

        if event::poll(Duration::from_millis(100))? {
            let mut event = event::read()?;

            match event {
                Event::Key(
                    ref mut key @ KeyEvent {
                        code: KeyCode::Char('h'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    },
                ) => {
                    key.code = KeyCode::Backspace;
                    key.modifiers = KeyModifiers::NONE;
                }
                _ => {}
            }

            match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Tab, ..
                }) => {
                    active_editor += 1;
                    active_editor %= editors.len();
                    terminal_size = None;
                    continue;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => return Ok(()),
                Event::Key(KeyEvent {
                    code: KeyCode::PageUp,
                    modifiers: KeyModifiers::ALT,
                    ..
                }) => {
                    terminal_size = None;
                    scroll = scroll.saturating_sub(5);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::PageDown,
                    modifiers: KeyModifiers::ALT,
                    ..
                }) => {
                    terminal_size = None;
                    scroll += 5;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    terminal_size = None;
                    debug ^= true;
                }
                _ => {
                    if editors[active_editor].handle_event(event) {
                        terminal_size = None;
                    }
                }
            }
        }
    }
}
