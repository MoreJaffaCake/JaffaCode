use super::*;

const HARD_LIMIT: usize = 200;
static HSPACES: &str = "                                                                                                                                                                                                        ";
static VSPACES: &str = "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n";

#[derive(derive_more::Debug)]
pub struct Buffer {
    #[debug(skip)]
    pub buffer_key: BufferKey,
    pub wrap_at: usize,
}

impl Buffer {
    pub fn new(buffer_key: BufferKey, wrap_at: usize) -> Self {
        debug_assert!(wrap_at < HARD_LIMIT);
        Self {
            buffer_key,
            wrap_at,
        }
    }

    pub fn insert_char(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        view: &mut View,
        c: char,
    ) {
        let Position {
            trailing_spaces,
            mut char_idx,
            newlines,
        } = *view.position(vlines, ropes);
        if newlines > 0 {
            let rope = view.cursor_rope_mut(vlines, ropes);
            rope.insert(char_idx, &VSPACES[..newlines]);
            vlines.insert_newlines(view, newlines);
            char_idx += newlines.saturating_sub(1);
        }
        if trailing_spaces > 0 {
            let rope = view.cursor_rope_mut(vlines, ropes);
            rope.insert(char_idx, &HSPACES[..trailing_spaces]);
            char_idx += trailing_spaces;
        }
        view.cursor_rope_mut(vlines, ropes).insert_char(char_idx, c);
        vlines.insert(view, trailing_spaces + 1, ropes, self.wrap_at);
        view.position(vlines, ropes).char_idx += 1;
        if c == '\n' || view.cur_x as usize + 1 >= self.wrap_at {
            view.move_cursor_next(vlines);
            view.cur_y += 1;
            view.cur_x = 0;
        } else {
            view.cur_x += 1;
        }
        let pos = view.position(vlines, ropes);
        pos.char_idx = char_idx + 1;
        pos.trailing_spaces = 0;
        pos.newlines = 0;
    }

    pub fn delete_char_forward(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        view: &mut View,
    ) {
        let Position {
            trailing_spaces,
            mut char_idx,
            ..
        } = *view.position(vlines, ropes);
        if char_idx >= view.cursor_rope_mut(vlines, ropes).len_chars() {
            return;
        }
        if trailing_spaces > 0 {
            view.cursor_rope_mut(vlines, ropes)
                .insert(char_idx, &HSPACES[..trailing_spaces]);
            vlines.insert(view, trailing_spaces, ropes, self.wrap_at);
            char_idx += trailing_spaces;
        }
        view.cursor_rope_mut(vlines, ropes)
            .remove(char_idx..=char_idx);
        vlines.remove(view, 1, ropes, self.wrap_at);
        let pos = view.position(vlines, ropes);
        pos.char_idx = char_idx;
        pos.trailing_spaces = 0;
    }

    pub fn delete_char_backward(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        view: &mut View,
    ) {
        let Position {
            mut char_idx,
            trailing_spaces,
            newlines,
        } = *view.position(vlines, ropes);
        if char_idx == 0 {
            return;
        } else if view.cur_x > 0 {
            view.cur_x -= 1;
        } else if view.cur_y > 0 {
            view.cur_y -= 1;
            if newlines == 0 {
                char_idx -= 1;
                view.move_cursor_prev(vlines);
                view.cur_x = view.line_len(vlines, ropes);
                let rope = view.cursor_rope_mut(vlines, ropes);
                rope.remove(char_idx..=char_idx);
                vlines.remove(view, 1, ropes, self.wrap_at);
                view.position(vlines, ropes).char_idx = char_idx;
            } else {
                view.position(vlines, ropes).newlines -= 1;
                if newlines == 1 {
                    view.cur_x = view.line_len(vlines, ropes);
                }
            }
            return;
        } else {
            view.scroll_up(vlines);
            view.move_cursor_prev(vlines);
            let line_len = view.line_len(vlines, ropes);
            view.cur_x = line_len as _;
        }
        if trailing_spaces == 0 {
            let rope = view.cursor_rope_mut(vlines, ropes);
            if char_idx == rope.len_chars() {
                char_idx -= 1;
            }
            char_idx -= 1;
            rope.remove(char_idx..=char_idx);
            vlines.remove(view, 1, ropes, self.wrap_at);
            view.position(vlines, ropes).char_idx = char_idx;
        } else {
            view.position(vlines, ropes).trailing_spaces -= 1;
        }
    }
}
