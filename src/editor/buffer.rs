use super::*;

#[derive(derive_more::Debug)]
pub struct Buffer {
    #[debug(skip)]
    pub start: VLineKey,
    pub end: VLineKey,
    pub wrap_at: usize,
    pub indent: usize,
}

impl Buffer {
    pub fn new(start: VLineKey, end: VLineKey, wrap_at: usize, indent: usize) -> Self {
        debug_assert!(wrap_at < MAX_WRAP_AT);
        Self {
            start,
            end,
            wrap_at,
            indent,
        }
    }

    pub fn insert_char(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        window: &mut Window,
        c: char,
    ) {
        let Position {
            trailing_spaces,
            mut char_idx,
            newlines,
        } = *window.position(vlines, ropes);
        let len_utf8 = c.len_utf8();
        if newlines > 0 {
            let rope = window.cursor_rope_mut(vlines, ropes);
            rope.insert(char_idx, &VSPACES[..newlines]);
            vlines.insert_newlines(window, newlines);
            char_idx += newlines.saturating_sub(1);
        }
        if trailing_spaces > 0 {
            let rope = window.cursor_rope_mut(vlines, ropes);
            rope.insert(char_idx, &HSPACES[..trailing_spaces]);
            char_idx += trailing_spaces;
        }
        window
            .cursor_rope_mut(vlines, ropes)
            .insert_char(char_idx, c);
        vlines.insert(window, trailing_spaces + len_utf8, ropes, self.wrap_at);
        window.position(vlines, ropes).char_idx += len_utf8;
        if c == '\n' || window.cur_x as usize + 1 >= self.wrap_at {
            window.move_cursor_next(vlines);
            window.cur_y += 1;
            window.cur_x = 0;
        } else {
            window.cur_x += 1;
        }
        let pos = window.position(vlines, ropes);
        pos.char_idx = char_idx + 1;
        pos.trailing_spaces = 0;
        pos.newlines = 0;
    }

    pub fn delete_char_forward(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        window: &mut Window,
    ) {
        let Position {
            trailing_spaces,
            mut char_idx,
            ..
        } = *window.position(vlines, ropes);
        if char_idx >= window.cursor_rope_mut(vlines, ropes).len_chars() {
            return;
        }
        if trailing_spaces > 0 {
            window
                .cursor_rope_mut(vlines, ropes)
                .insert(char_idx, &HSPACES[..trailing_spaces]);
            vlines.insert(window, trailing_spaces, ropes, self.wrap_at);
            char_idx += trailing_spaces;
        }
        window
            .cursor_rope_mut(vlines, ropes)
            .remove(char_idx..=char_idx);
        // TODO not 1 but the size of the char
        vlines.remove(window, 1, ropes, self.wrap_at);
        let pos = window.position(vlines, ropes);
        pos.char_idx = char_idx;
        pos.trailing_spaces = 0;
    }

    pub fn delete_char_backward(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        window: &mut Window,
    ) {
        let Position {
            mut char_idx,
            trailing_spaces,
            newlines,
        } = *window.position(vlines, ropes);
        if char_idx == 0 {
            return;
        } else if window.cur_x > 0 {
            window.cur_x -= 1;
        } else if window.cur_y > 0 {
            window.cur_y -= 1;
            if newlines == 0 {
                char_idx -= 1;
                window.move_cursor_prev(vlines);
                window.cur_x = window.line_len(vlines, ropes);
                let rope = window.cursor_rope_mut(vlines, ropes);
                rope.remove(char_idx..=char_idx);
                // TODO not 1 but the size of the char
                vlines.remove(window, 1, ropes, self.wrap_at);
                window.position(vlines, ropes).char_idx = char_idx;
            } else {
                window.position(vlines, ropes).newlines -= 1;
                if newlines == 1 {
                    window.cur_x = window.line_len(vlines, ropes);
                }
            }
            return;
        } else {
            window.scroll_up(vlines);
            window.move_cursor_prev(vlines);
            let line_len = window.line_len(vlines, ropes);
            window.cur_x = line_len as _;
        }
        if trailing_spaces == 0 {
            let rope = window.cursor_rope_mut(vlines, ropes);
            if char_idx == rope.len_chars() {
                char_idx -= 1;
            }
            char_idx -= 1;
            rope.remove(char_idx..=char_idx);
            // TODO not 1 but the size of the char
            vlines.remove(window, 1, ropes, self.wrap_at);
            window.position(vlines, ropes).char_idx = char_idx;
        } else {
            window.position(vlines, ropes).trailing_spaces -= 1;
        }
    }
}
