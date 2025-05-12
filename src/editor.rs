#[cfg(feature = "crossterm")]
mod crossterm;
mod visual_lines;

use ropey::*;

#[derive(Debug)]
pub struct Editor {
    rope: Rope,
    position: Option<Position>,
    cur_y: u16,
    cur_x: u16,
    white_spaces: String,
    vlines: visual_lines::VisualLines,
    wrap_at: usize,
}

#[derive(Debug, Clone, Copy)]
struct Position {
    trailing_spaces: usize,
    char_idx: usize,
}

impl Editor {
    pub fn new(initial_text: &str) -> Self {
        let mut instance = Self {
            rope: Rope::from_str(initial_text),
            position: None,
            cur_y: 0,
            cur_x: 0,
            white_spaces: std::iter::repeat(' ').take(200).collect::<String>(),
            vlines: Default::default(),
            wrap_at: 40,
        };
        let len_chars = instance.rope.len_chars();
        if instance.rope.char(len_chars - 1) != '\n' {
            instance.rope.insert_char(len_chars, '\n');
        }
        instance.vlines.regenerate(&instance.rope, instance.wrap_at);
        instance
    }

    fn position(&mut self) -> &mut Position {
        if self.position.is_none() {
            let line = self.vlines.cursor();
            let mut char_idx = self.rope.byte_to_char(line.start);
            let trailing_spaces;
            let line_len = line.slice(&self.rope).len_chars().saturating_sub(1);
            if line_len >= self.cur_x as usize {
                char_idx += self.cur_x as usize;
                trailing_spaces = 0;
            } else {
                char_idx += line_len;
                trailing_spaces = self.cur_x as usize - line_len;
            }
            self.position = Some(Position {
                trailing_spaces,
                char_idx,
            });
        }

        self.position.as_mut().unwrap()
    }

    #[inline(always)]
    fn clear_position(&mut self) {
        self.position.take();
    }

    pub fn insert_char(&mut self, c: char) {
        let Position {
            trailing_spaces,
            mut char_idx,
        } = *self.position();
        if trailing_spaces > 0 {
            self.rope
                .insert(char_idx, &self.white_spaces[..trailing_spaces]);
            char_idx += trailing_spaces;
        }
        self.rope.insert_char(char_idx, c);
        self.position().char_idx += 1;
        if c == '\n' {
            self.vlines.move_cursor_next();
            self.cur_y += 1;
            self.cur_x = 0;
        } else {
            self.cur_x += 1;
        }
        self.vlines
            .insert(trailing_spaces + 1, &self.rope, self.wrap_at);
        let pos = self.position();
        pos.char_idx = char_idx + 1;
        pos.trailing_spaces = 0;
    }

    pub fn delete_char_forward(&mut self) {
        let char_idx = self.position().char_idx;
        if self.rope.try_remove(char_idx..=char_idx).is_ok() {
            self.vlines.remove(1);
        }
    }

    pub fn delete_char_backward(&mut self) {
        let Position { mut char_idx, .. } = *self.position();
        if self.cur_x > 0 {
            char_idx -= 1;
            self.rope.remove(char_idx..=char_idx);
            self.vlines.remove(1);
            self.cur_x -= 1;
            self.position().char_idx = char_idx;
        } else if char_idx > 0 {
            char_idx -= 1;
            self.rope.remove(char_idx..=char_idx);
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
            self.position().char_idx = char_idx;
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cur_y > 0 {
            self.cur_y -= 1;
            self.vlines.move_cursor_prev();
            self.clear_position();
        }
    }

    pub fn move_cursor_down(&mut self) {
        self.cur_y += 1;
        self.vlines.move_cursor_next();
        self.clear_position();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cur_x > 0 {
            self.cur_x -= 1;
            self.clear_position();
        }
    }

    pub fn move_cursor_right(&mut self) {
        self.cur_x += 1;
        self.clear_position();
    }

    pub fn get_display_lines(&self) -> impl Iterator<Item = RopeSlice> {
        self.vlines.iter().map(|line| line.slice(&self.rope))
    }

    pub fn cursor_position<T: From<u16>>(&self) -> (T, T) {
        (T::from(self.cur_x), T::from(self.cur_y))
    }
}
