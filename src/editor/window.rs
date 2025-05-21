use super::*;

#[derive(derive_more::Debug)]
pub struct Window {
    #[debug(skip)]
    start: VLineKey,
    start_idx: usize,
    #[debug(skip)]
    end: VLineKey,
    #[debug(skip)]
    cursor: VLineKey,
    cursor_idx: usize,
    position: Option<Position>,
    cur_y: u16,
    cur_x: u16,
    indent: usize,
}

#[derive(Debug, Clone, Copy)]
struct Position {
    char_idx: usize,
    trailing_spaces: usize,
    newlines: usize,
    relative_x: usize,
    invalid: bool,
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

    #[inline]
    pub fn get_display_lines<'r>(
        &self,
        vlines: &VLines,
        ropes: &'r RopeMap,
        buffers: &BufferMap,
    ) -> impl Iterator<Item = DisplayLine<'r>> {
        vlines.slices(ropes, buffers, self.start, self.end, self.indent)
    }

    #[inline(always)]
    pub fn cursor(&self) -> VLineKey {
        self.cursor
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

    fn move_cursor_prev(&mut self, vlines: &VLines) -> bool {
        let prev = vlines[self.cursor].prev;
        if self.cursor_idx > 0 && vlines.contains_key(prev) {
            self.cursor = prev;
            self.cursor_idx -= 1;
            true
        } else {
            false
        }
    }

    fn move_cursor_next(&mut self, vlines: &VLines) -> bool {
        let next = vlines[self.cursor].next;
        if next != self.end && vlines.contains_key(next) {
            self.cursor = next;
            self.cursor_idx += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    fn position(&mut self, vlines: &VLines, ropes: &RopeMap, buffers: &BufferMap) -> Position {
        if self.position.is_none() {
            self.position = Some(self.get_position(vlines, ropes, buffers));
        }
        debug_assert!({
            let p = self.position.unwrap();
            dbg!(p);
            (p.relative_x > 0 && self.cur_x > 0) || p.relative_x == 0
        });
        self.position.unwrap()
    }

    #[inline]
    fn with_position(&mut self, f: impl Fn(&mut Position)) {
        if let Some(mut position) = self.position.take() {
            f(&mut position);
            self.position = Some(position);
        }
    }

    #[inline(always)]
    fn clear_position(&mut self) {
        self.position = None;
    }

    fn get_position(&self, vlines: &VLines, ropes: &RopeMap, buffers: &BufferMap) -> Position {
        let line = &vlines[self.cursor];
        let indent = buffers[line.buffer_key].indent;
        let mut invalid = false;
        let relative_x = self.cur_x as usize + self.indent;
        let relative_x = relative_x.checked_sub(indent).unwrap_or_else(|| {
            invalid = true;
            0
        });
        let rope = &ropes[line.buffer_key];
        let mut char_idx = rope.byte_to_char(line.start_byte);
        let newlines = (self.start_idx + self.cur_y as usize).saturating_sub(self.cursor_idx);
        let trailing_spaces: usize;
        let len_chars = line.slice(ropes).len_chars().saturating_sub(1);
        if newlines > 0 {
            char_idx = rope.byte_to_char(line.end_byte);
            trailing_spaces = relative_x as usize;
        } else if len_chars >= relative_x as usize {
            char_idx += relative_x as usize;
            trailing_spaces = 0;
        } else {
            char_idx += len_chars;
            trailing_spaces = relative_x - len_chars;
        }
        Position {
            trailing_spaces,
            char_idx,
            newlines,
            relative_x,
            invalid,
        }
    }

    #[inline(always)]
    fn slice<'r>(&self, vlines: &VLines, ropes: &'r RopeMap) -> RopeSlice<'r> {
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

    pub fn move_cursor_down(&mut self, vlines: &VLines, limit: u16) {
        if self.cur_y < limit {
            self.cur_y += 1;
            self.move_cursor_next(vlines);
            self.clear_position();
        } else {
            self.scroll_down(vlines);
        }
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
            self.cur_x = vlines[self.cursor]
                .slice(ropes)
                .len_chars()
                .saturating_sub(1) as _;
        }
        self.clear_position();
    }

    pub fn move_cursor_right(&mut self, vlines: &VLines, buffers: &BufferMap) {
        let buffer = &buffers[vlines[self.cursor].buffer_key];
        if self.cur_x as usize + 1 < buffer.wrap_at + buffer.indent {
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

    pub fn move_cursor_at_start(&mut self, vlines: &VLines, ropes: &RopeMap, buffers: &BufferMap) {
        self.cur_x = self
            .slice(vlines, ropes)
            .chars()
            .enumerate()
            .find_map(|(i, c)| (!c.is_whitespace()).then_some(i as u16))
            .unwrap_or(0);
        // TODO cursor might be outside the text
        let buffer = &buffers[vlines[self.cursor].buffer_key];
        self.cur_x += buffer.indent.saturating_sub(self.indent) as u16;
        self.clear_position();
    }

    pub fn move_cursor_at_end(&mut self, vlines: &VLines, ropes: &RopeMap, buffers: &BufferMap) {
        let slice = self.slice(vlines, ropes);
        let len_chars = slice.len_chars();
        self.cur_x = slice
            .chars_at(len_chars)
            .reversed()
            .enumerate()
            .find_map(|(i, c)| (!c.is_whitespace()).then_some((len_chars - i) as u16))
            .unwrap_or(0);
        // TODO cursor might be outside the text
        let buffer = &buffers[vlines[self.cursor].buffer_key];
        self.cur_x += buffer.indent.saturating_sub(self.indent) as u16;
        self.clear_position();
    }

    pub fn delete_char_forward(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        buffers: &BufferMap,
    ) {
        let Position {
            trailing_spaces,
            mut char_idx,
            invalid,
            ..
        } = self.position(vlines, ropes, buffers);
        let line = &vlines[self.cursor];
        let buffer = &buffers[line.buffer_key];
        if char_idx >= ropes[line.buffer_key].len_chars() || invalid {
            return;
        }
        if trailing_spaces > 0 {
            buffer.insert(
                vlines,
                ropes,
                char_idx,
                &HSPACES[..trailing_spaces],
                self.cursor,
            );
            char_idx += trailing_spaces;
        }
        buffer.remove(vlines, ropes, char_idx, self.cursor);
        self.with_position(|p| {
            p.char_idx = char_idx;
            p.trailing_spaces = 0;
        });
    }

    pub fn insert_char(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        buffers: &BufferMap,
        c: char,
    ) {
        let Position {
            trailing_spaces,
            mut char_idx,
            newlines,
            mut relative_x,
            invalid,
            ..
        } = self.position(vlines, ropes, buffers);
        if invalid {
            return;
        }
        let line = &vlines[self.cursor];
        let buffer = &buffers[line.buffer_key];
        if newlines > 0 {
            self.cursor = buffer.insert(vlines, ropes, char_idx, &VSPACES[..newlines], self.cursor);
            char_idx += newlines - 1;
            self.cursor_idx += newlines;
        }
        if trailing_spaces > 0 {
            buffer.insert(
                vlines,
                ropes,
                char_idx,
                &HSPACES[..trailing_spaces],
                self.cursor,
            );
            char_idx += trailing_spaces;
        }
        buffer.insert_char(vlines, ropes, char_idx, c, self.cursor);
        char_idx += 1;
        if c == '\n' {
            self.move_cursor_next(vlines);
            self.cur_y += 1;
            self.cur_x = (buffer.indent - self.indent) as _;
            relative_x = 0;
        } else if relative_x + 1 > buffer.wrap_at {
            self.move_cursor_next(vlines);
            self.cur_y += 1;
            self.cur_x = (buffer.indent + 1 - self.indent) as _;
            relative_x = 1;
        } else {
            self.cur_x += 1;
            relative_x += 1;
        }
        self.with_position(|p| {
            p.char_idx = char_idx;
            p.trailing_spaces = 0;
            p.newlines = 0;
            p.relative_x = relative_x;
        });
    }

    pub fn delete_char_backward(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        buffers: &BufferMap,
    ) {
        let Position {
            mut char_idx,
            trailing_spaces,
            newlines,
            relative_x,
            invalid,
        } = self.position(vlines, ropes, buffers);
        let line = &vlines[self.cursor];
        let buffer = &buffers[line.buffer_key];
        if char_idx == 0 || invalid {
            return;
        } else if relative_x > 0 {
            self.cur_x -= 1;
            self.with_position(|p| p.relative_x -= 1);
        } else if self.cur_y > 0 {
            self.cur_y -= 1;
            char_idx -= 1;
            if newlines == 0 {
                self.move_cursor_prev(vlines);
                let line = &vlines[self.cursor];
                let len_chars = line.slice(ropes).len_chars().saturating_sub(1);
                self.cur_x = (len_chars + buffer.indent - self.indent) as u16;
                buffer.remove(vlines, ropes, char_idx, self.cursor);
                self.with_position(|p| {
                    p.char_idx = char_idx;
                    p.relative_x = len_chars;
                });
            } else {
                self.with_position(|p| p.newlines -= 1);
                if newlines == 1 {
                    let line = &vlines[self.cursor];
                    let len_chars = line.slice(ropes).len_chars().saturating_sub(1);
                    self.cur_x = (len_chars + buffer.indent - self.indent) as u16;
                    self.with_position(|p| {
                        p.char_idx = char_idx;
                        p.relative_x = len_chars;
                    });
                }
            }
            return;
        } else {
            self.scroll_up(vlines);
            self.move_cursor_prev(vlines);
            let line = &vlines[self.cursor];
            let len_chars = line.slice(ropes).len_chars().saturating_sub(1);
            self.cur_x = (len_chars + buffer.indent - self.indent) as u16;
        }
        if trailing_spaces == 0 {
            char_idx -= 1;
            buffer.remove(vlines, ropes, char_idx, self.cursor);
            self.with_position(|p| p.char_idx = char_idx);
        } else {
            self.with_position(|p| p.trailing_spaces -= 1);
        }
    }
}
