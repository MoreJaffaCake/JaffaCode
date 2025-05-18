use super::*;

#[derive(derive_more::Debug)]
pub struct Window {
    #[debug(skip)]
    pub start: VLineKey,
    pub start_idx: usize,
    #[debug(skip)]
    pub end: VLineKey,
    #[debug(skip)]
    pub cursor: VLineKey,
    pub cursor_idx: usize,
    pub position: Option<Position>,
    pub cur_y: u16,
    pub cur_x: u16,
    pub indent: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub char_idx: usize,
    pub trailing_spaces: usize,
    pub newlines: usize,
}

impl Window {
    pub fn new(buffers: &BufferMap, vlines: &VLines, start: VLineKey, end: VLineKey) -> Self {
        Self {
            start,
            start_idx: 0,
            end,
            cursor: start,
            cursor_idx: 0,
            position: None,
            cur_y: 0,
            cur_x: 0,
            indent: buffers[vlines[start].buffer_key].indent,
        }
    }

    pub fn cursor_rope_mut<'r>(&self, vlines: &VLines, ropes: &'r mut RopeMap) -> &'r mut Rope {
        &mut ropes[vlines[self.cursor].buffer_key]
    }

    pub fn scroll_up(&mut self, vlines: &VLines) -> bool {
        let prev = vlines[self.start].prev;
        if self.start_idx > 0 && vlines.contains_key(prev) {
            self.start = prev;
            self.start_idx -= 1;
            self.move_cursor_prev(vlines);
            self.clear_position();
            true
        } else {
            false
        }
    }

    pub fn scroll_down(&mut self, vlines: &VLines) -> bool {
        let next = vlines[self.start].next;
        if next != self.end && vlines.contains_key(next) {
            self.start = next;
            self.start_idx += 1;
            self.move_cursor_next(vlines);
            self.clear_position();
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn move_cursor_prev(&mut self, vlines: &VLines) -> bool {
        let prev = vlines[self.cursor].prev;
        if self.cursor_idx > 0 && vlines.contains_key(prev) {
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
        if next != self.end && vlines.contains_key(next) {
            self.cursor = next;
            self.cursor_idx += 1;
            true
        } else {
            false
        }
    }

    // TODO indented buffer need to be de-indented OR re-implement hscroll?
    pub fn position(&mut self, vlines: &VLines, ropes: &RopeMap) -> &mut Position {
        if self.position.is_none() {
            let line = &vlines[self.cursor];
            let rope = &ropes[line.buffer_key];
            let mut char_idx = rope.byte_to_char(line.start_byte);
            let newlines = (self.start_idx + self.cur_y as usize).saturating_sub(self.cursor_idx);
            let trailing_spaces: usize;
            let len_chars = line.slice(ropes).len_chars();
            if newlines > 0 {
                char_idx = rope.byte_to_char(line.end_byte);
                trailing_spaces = self.cur_x as usize;
            } else if len_chars >= self.cur_x as usize {
                char_idx += self.cur_x as usize;
                trailing_spaces = 0;
            } else {
                let line_len = len_chars.saturating_sub(1);
                char_idx += line_len;
                trailing_spaces = self.cur_x as usize - line_len;
            }
            self.position = Some(Position {
                trailing_spaces,
                char_idx,
                newlines,
            });
        }
        dbg!(self.position.as_ref().unwrap());
        self.position.as_mut().unwrap()
    }

    #[inline(always)]
    pub fn clear_position(&mut self) {
        self.position = None;
    }

    #[inline(always)]
    pub fn line_len(&self, vlines: &VLines, ropes: &RopeMap) -> u16 {
        vlines[self.cursor]
            .slice(ropes)
            .len_chars()
            .saturating_sub(1) as _
    }

    pub fn slice<'r>(&self, vlines: &VLines, ropes: &'r RopeMap) -> RopeSlice<'r> {
        vlines[self.cursor].slice(ropes)
    }

    pub fn cursor_position<T: From<u16>>(&self) -> (T, T) {
        (T::from(self.cur_x), T::from(self.cur_y))
    }

    pub fn move_cursor_up(&mut self, vlines: &VLines) {
        if self.cur_y > 0 {
            self.cur_y -= 1;
            if self.cursor_idx > self.cur_y as usize + self.start_idx {
                self.move_cursor_prev(vlines);
            }
        } else {
            // TODO scrolling even before the file so we can add text there?
            self.scroll_up(vlines);
        }
        self.clear_position();
    }

    pub fn move_cursor_down(&mut self, vlines: &VLines) {
        self.cur_y += 1;
        self.move_cursor_next(vlines);
        self.clear_position();
    }

    pub fn move_cursor_left(&mut self, vlines: &VLines, ropes: &RopeMap) {
        if self.cur_x > 0 {
            self.cur_x -= 1;
        } else {
            if self.cur_y > 0 {
                self.cur_y -= 1;
                self.move_cursor_prev(vlines);
            } else if self.start_idx > 0 {
                self.scroll_up(vlines);
            } else {
                return;
            }
            self.cur_x = self.line_len(vlines, ropes);
        }
        self.clear_position();
    }

    pub fn move_cursor_right(&mut self, vlines: &VLines, buffers: &BufferMap) {
        let buffer = &buffers[vlines[self.cursor].buffer_key];
        if self.cur_x as usize + 1 < buffer.wrap_at {
            self.cur_x += 1;
        } else {
            self.cur_x = 0;
            self.cur_y += 1;
            self.move_cursor_next(vlines);
        }
        self.clear_position();
    }

    pub fn move_cursor_at_0(&mut self) {
        self.cur_x = 0;
        self.clear_position();
    }

    pub fn move_cursor_at_start(&mut self, vlines: &VLines, ropes: &RopeMap) {
        let new_cur_x = self
            .slice(vlines, ropes)
            .chars()
            .enumerate()
            .find_map(|(i, c)| (!c.is_whitespace()).then_some(i as u16))
            .unwrap_or(0);
        self.cur_x = new_cur_x;
        self.clear_position();
    }

    pub fn move_cursor_at_end(&mut self, vlines: &VLines, ropes: &RopeMap) {
        let slice = self.slice(vlines, ropes);
        let len_chars = slice.len_chars();
        self.cur_x = slice
            .chars_at(len_chars)
            .reversed()
            .enumerate()
            .find_map(|(i, c)| (!c.is_whitespace()).then_some((len_chars - i) as u16))
            .unwrap_or(0);
        self.clear_position();
    }
}
