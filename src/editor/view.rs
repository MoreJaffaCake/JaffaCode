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
            hscroll: 0,
        }
    }

    #[inline(always)]
    pub fn scroll_up(&mut self, vlines: &VLines) {
        self.start = vlines[self.start].prev;
        debug_assert!(vlines.contains_key(self.start));
        self.start_idx -= 1;
    }

    #[inline(always)]
    pub fn scroll_down(&mut self, vlines: &VLines) {
        let next = vlines[self.start].next;
        if vlines.contains_key(next) {
            self.start = next;
            self.start_idx += 1;
        }
    }

    #[inline(always)]
    pub fn move_cursor_prev(&mut self, vlines: &VLines) {
        self.cursor = vlines[self.cursor].prev;
        debug_assert!(vlines.contains_key(self.cursor));
        self.cursor_idx -= 1;
    }

    #[inline(always)]
    pub fn move_cursor_next(&mut self, vlines: &VLines) {
        let next = vlines[self.cursor].next;
        if vlines.contains_key(next) {
            self.cursor = next;
            self.cursor_idx += 1;
        }
    }

    #[inline(always)]
    pub fn scroll_left(&mut self, i: usize) {
        self.hscroll = self.hscroll.saturating_sub(i);
    }

    #[inline(always)]
    pub fn scroll_right(&mut self, i: usize) {
        self.hscroll += i;
    }

    pub fn get_position(&mut self, x: usize, y: usize, vlines: &VLines, rope: &Rope) -> Position {
        let line = &vlines[self.cursor];
        let mut char_idx = rope.byte_to_char(line.start_byte) + self.hscroll;
        let newlines = (self.start_idx + y).saturating_sub(self.cursor_idx);
        let trailing_spaces;
        let line_len = line.slice(rope).len_chars().saturating_sub(1);
        if newlines > 0 {
            char_idx = rope.byte_to_char(line.end);
            trailing_spaces = x + self.hscroll;
        } else if line_len >= x {
            char_idx += x;
            trailing_spaces = 0;
        } else {
            char_idx += line_len;
            trailing_spaces = x - line_len + self.hscroll;
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
}
