use super::*;
use ::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

impl Editor {
    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('0'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.move_cursor_at_0();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            }) if modifiers == KeyModifiers::SHIFT || modifiers == KeyModifiers::NONE => {
                self.insert_char(c);
            }
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            }) => self.insert_char('\n'),
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.delete_char_backward();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Delete,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.delete_char_forward();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.move_cursor_up();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.move_cursor_down();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.move_cursor_left();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.move_cursor_right();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::SHIFT,
                ..
            }) => {
                self.scroll_up();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::SHIFT,
                ..
            }) => {
                self.scroll_down();
            }
            Event::Key(KeyEvent {
                code: KeyCode::PageUp,
                ..
            }) => {
                self.page_up();
            }
            Event::Key(KeyEvent {
                code: KeyCode::PageDown,
                ..
            }) => {
                self.page_down();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Home,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.move_cursor_at_start();
            }
            Event::Key(KeyEvent {
                code: KeyCode::End,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.move_cursor_at_end();
            }
            Event::Key(KeyEvent {
                code: KeyCode::F(8),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.split_buffer();
            }
            Event::Key(KeyEvent {
                code: KeyCode::F(7),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.create_window();
            }
            Event::Key(KeyEvent {
                code: KeyCode::F(6),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.root_window();
            }
            Event::Mouse(_) => {}
            _ => {
                dbg!(event);
            }
        }
    }
}
