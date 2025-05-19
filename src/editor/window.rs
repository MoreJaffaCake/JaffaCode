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
    position: Option<Position>,
    pub cur_y: u16,
    pub cur_x: u16,
    pub indent: usize,
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

    #[inline(always)]
    fn position(&mut self, vlines: &VLines, ropes: &RopeMap, buffers: &BufferMap) -> Position {
        if self.position.is_none() {
            self.position = Some(self.get_position(vlines, ropes, buffers));
        }
        dbg!(self.position.unwrap())
    }

    #[inline(always)]
    fn position_as_mut(&mut self) -> &mut Position {
        self.position.as_mut().unwrap()
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
        let len_chars = line.slice(ropes).len_chars();
        if newlines > 0 {
            char_idx = rope.byte_to_char(line.end_byte);
            trailing_spaces = relative_x as usize;
        } else if len_chars >= relative_x as usize {
            char_idx += relative_x as usize;
            trailing_spaces = 0;
        } else {
            let line_len = len_chars.saturating_sub(1);
            char_idx += line_len;
            trailing_spaces = relative_x as usize - line_len;
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

    #[inline(always)]
    fn line_len(&self, vlines: &VLines, ropes: &RopeMap, buffers: &BufferMap) -> usize {
        let line = &vlines[self.cursor];
        let buffer = &buffers[line.buffer_key];
        let line_len = line.slice(ropes).len_chars().saturating_sub(1);
        line_len + buffer.indent - self.indent
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

    pub fn move_cursor_left(&mut self, vlines: &VLines, ropes: &RopeMap, buffers: &BufferMap) {
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
            self.cur_x = Buffer::line_len(vlines, ropes, buffers, self.cursor) as _;
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
        let position = self.position_as_mut();
        position.char_idx = char_idx;
        position.trailing_spaces = 0;
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
            buffer.insert(vlines, ropes, char_idx, &VSPACES[..newlines], self.cursor);
            char_idx += newlines.saturating_sub(1);
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
            relative_x = 1;
        } else if relative_x + 1 > buffer.wrap_at {
            self.move_cursor_next(vlines);
            self.cur_y += 1;
            self.cur_x = (buffer.indent + 1 - self.indent) as _;
            relative_x = 1;
        } else {
            self.cur_x += 1;
            relative_x += 1;
        }
        let pos = self.position_as_mut();
        pos.char_idx = char_idx;
        pos.trailing_spaces = 0;
        pos.newlines = 0;
        pos.relative_x = relative_x;
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
            self.position_as_mut().relative_x -= 1;
        } else if self.cur_y > 0 {
            self.cur_y -= 1;
            if newlines == 0 {
                char_idx -= 1;
                self.move_cursor_prev(vlines);
                let line_len = self.line_len(vlines, ropes, buffers);
                self.cur_x = line_len as u16;
                buffer.remove(vlines, ropes, char_idx, self.cursor);
                let position = self.position_as_mut();
                position.char_idx = char_idx;
                position.relative_x = line_len - buffer.indent;
            } else {
                self.position_as_mut().newlines -= 1;
                if newlines == 1 {
                    let line_len = self.line_len(vlines, ropes, buffers);
                    self.cur_x = line_len as u16;
                    self.position_as_mut().relative_x = line_len - buffer.indent;
                }
            }
            return;
        } else {
            self.scroll_up(vlines);
            self.move_cursor_prev(vlines);
            let line_len = self.line_len(vlines, ropes, buffers);
            self.cur_x = line_len as u16;
            self.position_as_mut().relative_x = line_len - buffer.indent;
        }
        if trailing_spaces == 0 {
            let rope = self.cursor_rope_mut(vlines, ropes);
            if char_idx == rope.len_chars() {
                char_idx -= 1;
            }
            char_idx -= 1;
            buffer.remove(vlines, ropes, char_idx, self.cursor);
            self.position_as_mut().char_idx = char_idx;
        } else {
            self.position_as_mut().trailing_spaces -= 1;
        }
    }
}
