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
        Editor::new(
            "Pane 1:
Edit this text.
Use PageUp/PageDown to switch.",
        ),
        Editor::new(
            "Pane 2:
Another text buffer.",
        ),
    ];
    let constraints = std::iter::repeat_n(Constraint::Fill(1), editors.len()).collect::<Vec<_>>();
    let mut active_editor = 0;
    let mut scroll: usize = 1;

    loop {
        terminal.draw(|f| {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Fill(1), Constraint::Fill(2)])
                .split(f.area());
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(&constraints)
                .split(cols[0]);

            for (i, editor) in editors.iter().enumerate() {
                let mut block = Block::default().borders(Borders::ALL);
                if i == active_editor {
                    block = block.title("Active");
                    let (x, y) = editor.cursor_position();
                    let inner = block.inner(chunks[i]);
                    let offset = inner.offset(Offset { x, y }).intersection(inner);
                    if !offset.is_empty() {
                        f.set_cursor_position(offset);
                    }
                }
                let p = Paragraph::new(
                    editor
                        .get_display_lines()
                        .map(|x| Line::from(Span::raw(x)))
                        .collect::<Vec<_>>(),
                )
                .block(block);
                f.render_widget(p, chunks[i]);
            }

            let debug = format!("{:#?}", editors);
            let p = Paragraph::new(
                debug
                    .lines()
                    .skip(scroll)
                    .map(|x| Line::from(Span::raw(x)))
                    .collect::<Vec<_>>(),
            )
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(p, cols[1]);
        })?;

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
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    active_editor += 1;
                    active_editor %= editors.len();
                    continue;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) => return Ok(()),
                Event::Key(KeyEvent {
                    code: KeyCode::PageUp,
                    ..
                }) => {
                    scroll = scroll.saturating_sub(10);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::PageDown,
                    ..
                }) => {
                    scroll += 10;
                }
                _ => {}
            }

            editors[active_editor].handle_event(event);
        }
    }
}
