use super::vlines::*;
use ropey::*;

#[derive(derive_more::Debug)]
pub struct View {
    #[debug(skip)]
    pub start: Key,
    pub start_idx: usize,
    #[debug(skip)]
    pub cursor: Key,
    pub cursor_idx: usize,
    pub hscroll: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub char_idx: usize,
    pub trailing_spaces: usize,
    pub newlines: usize,
}

impl View {
    pub fn new(start: Key) -> Self {
        Self {
            start,
            start_idx: 0,
            cursor: start,
            cursor_idx: 0,
            hscroll: 10,
        }
    }

    #[inline(always)]
    pub fn scroll_up(&mut self, vlines: &VLines) -> bool {
        let prev = vlines[self.start].prev;
        if vlines.contains_key(prev) {
            self.start = prev;
            self.start_idx -= 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn scroll_down(&mut self, vlines: &VLines) -> bool {
        let next = vlines[self.start].next;
        if vlines.contains_key(next) {
            self.start = next;
            self.start_idx += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn move_cursor_prev(&mut self, vlines: &VLines) -> bool {
        let prev = vlines[self.cursor].prev;
        if vlines.contains_key(prev) {
            self.cursor = prev;
            self.cursor_idx -= 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn move_cursor_next(&mut self, vlines: &VLines) -> bool {
        let next = vlines[self.cursor].next;
        if vlines.contains_key(next) {
            self.cursor = next;
            self.cursor_idx += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn scroll_left(&mut self, i: usize) -> bool {
        if let Some(hscroll) = self.hscroll.checked_sub(i) {
            self.hscroll = hscroll;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn scroll_right(&mut self, vlines: &VLines, i: usize) -> bool {
        if self.hscroll + i < vlines.wrap_at() {
            self.hscroll += i;
            true
        } else {
            false
        }
    }

    pub fn get_position(&mut self, x: usize, y: usize, vlines: &VLines, rope: &Rope) -> Position {
        let line = &vlines[self.cursor];
        let mut char_idx = rope.byte_to_char(line.start_byte);
        let newlines = (self.start_idx + y).saturating_sub(self.cursor_idx);
        let trailing_spaces;
        let line_len = line.slice(rope).len_chars().saturating_sub(1);
        if newlines > 0 {
            char_idx = rope.byte_to_char(line.end);
            trailing_spaces = x + self.hscroll;
        } else if line_len > x + self.hscroll {
            char_idx += x + self.hscroll;
            trailing_spaces = 0;
        } else {
            char_idx += line_len;
            trailing_spaces = x + self.hscroll - line_len;
        }
        Position {
            trailing_spaces,
            char_idx,
            newlines,
        }
    }

    #[inline(always)]
    pub fn len_chars(&self, vlines: &VLines, rope: &Rope) -> u16 {
        vlines[self.cursor]
            .slice(rope)
            .len_chars()
            .saturating_sub(1) as _
    }

    pub fn slice<'r>(&self, vlines: &VLines, rope: &'r Rope) -> RopeSlice<'r> {
        let slice = vlines[self.cursor].slice(rope);
        if self.hscroll > 0 {
            if slice.len_chars() <= self.hscroll {
                slice.slice(..0)
            } else {
                slice.slice(self.hscroll..)
            }
        } else {
            slice
        }
    }
}
