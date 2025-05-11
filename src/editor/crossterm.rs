use super::*;
use ::crossterm::event::{Event, KeyCode};

impl Editor {
    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char(c) => {
                    self.insert_char(c);
                }
                KeyCode::Enter => self.insert_char('\n'),
                KeyCode::Backspace => {
                    self.delete_char_backward();
                }
                KeyCode::Delete => {
                    self.delete_char_forward();
                }
                KeyCode::Up => {
                    self.move_cursor_up();
                }
                KeyCode::Down => {
                    self.move_cursor_down();
                }
                KeyCode::Left => {
                    self.move_cursor_left();
                }
                KeyCode::Right => {
                    self.move_cursor_right();
                }
                _ => {}
            }
        }
    }
}
