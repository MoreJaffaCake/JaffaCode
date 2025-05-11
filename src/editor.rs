#[cfg(feature = "crossterm")]
mod crossterm;
mod visual_lines;

use ropey::*;

#[derive(Debug)]
pub struct Editor {
    rope: Rope,
    trailing_spaces: usize,
    char_idx: usize,
    cur_y: u16,
    cur_x: u16,
    white_spaces: String,
    vlines: visual_lines::VisualLines,
}

impl Editor {
    pub fn new(initial_text: &str) -> Self {
        let mut instance = Self {
            rope: Rope::from_str(initial_text),
            trailing_spaces: 0,
            char_idx: 0,
            cur_y: 0,
            cur_x: 0,
            white_spaces: std::iter::repeat(' ').take(200).collect::<String>(),
            vlines: Default::default(),
        };
        let len_chars = instance.rope.len_chars();
        if instance.rope.char(len_chars - 1) != '\n' {
            instance.rope.insert_char(len_chars, '\n');
        }
        instance.vlines.regenerate(&instance.rope, 100);
        instance
    }

    pub fn insert_char(&mut self, c: char) {
        self.vlines.insert(self.trailing_spaces + 1);
        if self.trailing_spaces > 0 {
            self.rope
                .insert(self.char_idx, &self.white_spaces[..self.trailing_spaces]);
            self.char_idx += self.trailing_spaces;
            self.trailing_spaces = 0;
        }
        self.rope.insert_char(self.char_idx, c);
        self.char_idx += 1;
        if c == '\n' {
            self.vlines.move_cursor_next();
            self.cur_y += 1;
            self.cur_x = 0;
        } else {
            self.cur_x += 1;
        }
    }

    pub fn delete_char_forward(&mut self) {
        if self.rope.try_remove(self.char_idx..=self.char_idx).is_ok() {
            self.vlines.remove(1);
        }
    }

    pub fn delete_char_backward(&mut self) {
        if self.cur_x > 0 {
            self.char_idx -= 1;
            self.rope.remove(self.char_idx..=self.char_idx);
            self.vlines.remove(1);
            self.cur_x -= 1;
        } else if self.char_idx > 0 {
            self.char_idx -= 1;
            self.rope.remove(self.char_idx..=self.char_idx);
            self.vlines.remove(1);
            self.vlines.move_cursor_prev();
            let line_len = self
                .vlines
                .cursor()
                .slice(&self.rope)
                .len_chars()
                .saturating_sub(1);
            let line = self.vlines.merge_next();
            self.cur_y -= 1;
            self.cur_x = line_len as _;
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cur_y > 0 {
            self.cur_y -= 1;
            self.vlines.move_cursor_prev();
            let line = self.vlines.cursor();
            let line_len = line.slice(&self.rope).len_chars().saturating_sub(1);
            if line_len >= self.cur_x as usize {
                self.char_idx = self.rope.byte_to_char(line.start) + self.cur_x as usize;
            } else {
                self.char_idx = self.rope.byte_to_char(line.end) - 1;
                self.trailing_spaces = self.cur_x as usize - line_len + 1;
            }
        }
    }

    pub fn move_cursor_down(&mut self) {
        self.cur_y += 1;
        self.vlines.move_cursor_next();
        let line = self.vlines.cursor();
        let line_len = line.slice(&self.rope).len_chars().saturating_sub(1);
        self.char_idx = self.rope.byte_to_char(line.start);
        if line_len >= self.cur_x as usize {
            self.char_idx += self.cur_x as usize;
            self.trailing_spaces = 0;
        } else {
            self.char_idx += line_len;
            self.trailing_spaces = self.cur_x as usize - line_len + 1;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cur_x > 0 {
            self.cur_x -= 1;
            if self.trailing_spaces > 0 {
                self.trailing_spaces -= 1;
            } else {
                self.char_idx -= 1;
            }
        }
    }

    pub fn move_cursor_right(&mut self) {
        self.cur_x += 1;
        let line = self.vlines.cursor();
        let line_len = line.slice(&self.rope).len_chars().saturating_sub(1);
        if line_len >= self.cur_x as usize {
            self.char_idx += 1;
        } else {
            self.trailing_spaces += 1;
        }
    }

    pub fn get_display_lines(&self) -> impl Iterator<Item = RopeSlice> {
        self.vlines.iter().map(|line| line.slice(&self.rope))
    }

    pub fn cursor_position<T: From<u16>>(&self) -> (T, T) {
        (T::from(self.cur_x), T::from(self.cur_y))
    }
}
