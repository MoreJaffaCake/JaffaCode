#[cfg(feature = "crossterm")]
mod crossterm;
mod view;
mod vlines;

use ropey::*;
use view::*;
use vlines::*;

#[derive(derive_more::Debug)]
pub struct Editor {
    rope: Rope,
    vlines: VLines,
    view: View,
    position: Option<Position>,
    cur_y: u16,
    cur_x: u16,
    #[debug(skip)]
    hspaces: String,
    #[debug(skip)]
    vspaces: String,
}

impl Editor {
    pub fn new(initial_text: &str) -> Self {
        let mut rope = Rope::from_str(initial_text);
        let len_chars = rope.len_chars();
        if rope.char(len_chars - 1) != '\n' {
            rope.insert_char(len_chars, '\n');
        }
        let vlines = VLines::new(40, &rope);
        let mut view = View::new(vlines.start());
        view.scroll_down(&vlines);
        view.move_cursor_next(&vlines);
        Self {
            rope,
            vlines,
            view,
            position: None,
            cur_y: 0,
            cur_x: 0,
            hspaces: std::iter::repeat(' ').take(200).collect::<String>(),
            vspaces: std::iter::repeat('\n').take(200).collect::<String>(),
        }
    }

    fn position(&mut self) -> &mut Position {
        if self.position.is_none() {
            self.position = Some(self.view.get_position(
                self.cur_x as _,
                self.cur_y as _,
                &self.vlines,
                &self.rope,
            ));
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
            newlines,
        } = *self.position();
        if newlines > 0 {
            self.rope.insert(char_idx, &self.vspaces[..newlines]);
            self.vlines.insert_newlines(&mut self.view, newlines);
            char_idx += newlines.saturating_sub(1);
        }
        if trailing_spaces > 0 {
            self.rope.insert(char_idx, &self.hspaces[..trailing_spaces]);
            char_idx += trailing_spaces;
        }
        self.rope.insert_char(char_idx, c);
        self.vlines
            .insert(&self.view, trailing_spaces + 1, &self.rope);
        self.position().char_idx += 1;
        if c == '\n' || self.cur_x as usize + 1 >= self.vlines.wrap_at() {
            self.view.move_cursor_next(&self.vlines);
            self.cur_y += 1;
            self.cur_x = 0;
        } else {
            self.cur_x += 1;
        }
        let pos = self.position();
        pos.char_idx = char_idx + 1;
        pos.trailing_spaces = 0;
        pos.newlines = 0;
    }

    pub fn delete_char_forward(&mut self) {
        let Position {
            trailing_spaces,
            mut char_idx,
            ..
        } = *self.position();
        if char_idx >= self.rope.len_chars() {
            return;
        }
        if trailing_spaces > 0 {
            self.rope.insert(char_idx, &self.hspaces[..trailing_spaces]);
            self.vlines.insert(&self.view, trailing_spaces, &self.rope);
            char_idx += trailing_spaces;
        }
        self.rope.remove(char_idx..=char_idx);
        self.vlines.remove(&mut self.view, 1, &self.rope);
        let pos = self.position();
        pos.char_idx = char_idx;
        pos.trailing_spaces = 0;
    }

    pub fn delete_char_backward(&mut self) {
        let Position {
            mut char_idx,
            trailing_spaces,
            newlines,
        } = *self.position();
        if char_idx == 0 {
            return;
        } else if self.cur_x > 0 {
            self.cur_x -= 1;
        } else if self.view.hscroll > 0 {
            self.view.scroll_left(1);
        } else if self.cur_y > 0 {
            self.cur_y -= 1;
            if newlines == 0 {
                char_idx -= 1;
                self.view.move_cursor_prev(&self.vlines);
                let line_len = self.vlines[self.view.cursor]
                    .slice(&self.rope)
                    .len_chars()
                    .saturating_sub(1);
                self.cur_x = line_len as _;
                self.rope.remove(char_idx..=char_idx);
                self.vlines.remove(&mut self.view, 1, &self.rope);
                self.position().char_idx = char_idx;
            } else {
                self.position().newlines -= 1;
                if newlines == 1 {
                    let line_len = self.vlines[self.view.cursor]
                        .slice(&self.rope)
                        .len_chars()
                        .saturating_sub(1);
                    self.cur_x = line_len as _;
                }
            }
            return;
        } else {
            self.view.scroll_up(&self.vlines);
            self.view.move_cursor_prev(&self.vlines);
            let line_len = self.vlines[self.view.cursor]
                .slice(&self.rope)
                .len_chars()
                .saturating_sub(1);
            self.cur_x = line_len as _;
        }
        if trailing_spaces == 0 {
            char_idx -= 1;
            self.rope.remove(char_idx..=char_idx);
            self.vlines.remove(&mut self.view, 1, &self.rope);
            self.position().char_idx = char_idx;
        } else {
            self.position().trailing_spaces -= 1;
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cur_y > 0 {
            self.cur_y -= 1;
            if self.view.cursor_idx > self.cur_y as usize {
                self.view.move_cursor_prev(&self.vlines);
            }
            self.clear_position();
        }
    }

    pub fn move_cursor_down(&mut self) {
        self.cur_y += 1;
        self.view.move_cursor_next(&self.vlines);
        self.clear_position();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cur_x > 0 {
            self.cur_x -= 1;
            self.clear_position();
        } else if self.view.hscroll > 0 {
            self.view.scroll_left(1);
            self.clear_position();
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cur_x as usize + 1 < self.vlines.wrap_at() {
            self.cur_x += 1;
        } else {
            self.cur_x = 0;
            self.cur_y += 1;
            self.view.move_cursor_next(&self.vlines);
        }
        self.clear_position();
    }

    pub fn move_cursor_at_start(&mut self) {
        let new_cur_x = self.vlines[self.view.cursor]
            .slice(&self.rope)
            .chars()
            .enumerate()
            .find_map(|(i, c)| (!c.is_whitespace()).then_some(i as u16))
            .unwrap_or(0);
        if new_cur_x == self.cur_x {
            self.cur_x = 0;
        } else {
            self.cur_x = new_cur_x
        }
        self.clear_position();
    }

    pub fn move_cursor_at_end(&mut self) {
        let slice = self.vlines[self.view.cursor].slice(&self.rope);
        let len_chars = slice.len_chars();
        self.cur_x = slice
            .chars_at(len_chars)
            .reversed()
            .enumerate()
            .find_map(|(i, c)| (!c.is_whitespace()).then_some((len_chars - i) as u16))
            .unwrap_or(len_chars.saturating_sub(1) as u16);
        self.clear_position();
    }

    pub fn get_display_lines(&self) -> impl Iterator<Item = RopeSlice> {
        self.vlines.slices(&self.view, &self.rope)
    }

    pub fn cursor_position<T: From<u16>>(&self) -> (T, T) {
        (T::from(self.cur_x), T::from(self.cur_y))
    }
}
