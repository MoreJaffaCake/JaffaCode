use super::*;

const HARD_LIMIT: usize = 200;
static HSPACES: &str = "                                                                                                                                                                                                        ";
static VSPACES: &str = "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n";

#[derive(derive_more::Debug)]
pub struct Buffer {
    #[debug(skip)]
    pub rope_key: RopeKey,
    pub wrap_at: usize,
}

impl Buffer {
    pub fn new(rope_key: RopeKey, wrap_at: usize) -> Self {
        debug_assert!(wrap_at < HARD_LIMIT);
        Self { rope_key, wrap_at }
    }

    pub fn insert_char(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        view: &mut View,
        c: char,
    ) {
        let rope = &mut ropes[self.rope_key];
        let Position {
            trailing_spaces,
            mut char_idx,
            newlines,
        } = *view.position(vlines, rope);
        if newlines > 0 {
            rope.insert(char_idx, &VSPACES[..newlines]);
            vlines.insert_newlines(view, newlines);
            char_idx += newlines.saturating_sub(1);
        }
        if trailing_spaces > 0 {
            rope.insert(char_idx, &HSPACES[..trailing_spaces]);
            char_idx += trailing_spaces;
        }
        rope.insert_char(char_idx, c);
        vlines.insert(view, trailing_spaces + 1, rope, self.wrap_at);
        view.position(vlines, rope).char_idx += 1;
        if c == '\n' || view.cur_x as usize + 1 >= self.wrap_at {
            view.move_cursor_next(vlines);
            view.cur_y += 1;
            view.cur_x = 0;
        } else {
            view.cur_x += 1;
        }
        let pos = view.position(vlines, rope);
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
        let rope = &mut ropes[self.rope_key];
        let Position {
            trailing_spaces,
            mut char_idx,
            ..
        } = *view.position(vlines, rope);
        if char_idx >= rope.len_chars() {
            return;
        }
        if trailing_spaces > 0 {
            rope.insert(char_idx, &HSPACES[..trailing_spaces]);
            vlines.insert(view, trailing_spaces, rope, self.wrap_at);
            char_idx += trailing_spaces;
        }
        rope.remove(char_idx..=char_idx);
        vlines.remove(view, 1, rope, self.wrap_at);
        let pos = view.position(vlines, rope);
        pos.char_idx = char_idx;
        pos.trailing_spaces = 0;
    }

    pub fn delete_char_backward(
        &mut self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        view: &mut View,
    ) {
        let rope = &mut ropes[self.rope_key];
        let Position {
            mut char_idx,
            trailing_spaces,
            newlines,
        } = *view.position(vlines, rope);
        if char_idx == 0 {
            return;
        } else if view.cur_x > 0 {
            view.cur_x -= 1;
        } else if view.cur_y > 0 {
            view.cur_y -= 1;
            if newlines == 0 {
                char_idx -= 1;
                view.move_cursor_prev(vlines);
                view.cur_x = view.line_len(vlines, rope);
                rope.remove(char_idx..=char_idx);
                vlines.remove(view, 1, rope, self.wrap_at);
                view.position(vlines, rope).char_idx = char_idx;
            } else {
                view.position(vlines, rope).newlines -= 1;
                if newlines == 1 {
                    view.cur_x = view.line_len(vlines, rope);
                }
            }
            return;
        } else {
            view.scroll_up(vlines);
            view.move_cursor_prev(vlines);
            let line_len = vlines[view.cursor]
                .slice(rope)
                .len_chars()
                .saturating_sub(1);
            view.cur_x = line_len as _;
        }
        if trailing_spaces == 0 {
            if char_idx == rope.len_chars() {
                char_idx -= 1;
            }
            char_idx -= 1;
            rope.remove(char_idx..=char_idx);
            vlines.remove(view, 1, rope, self.wrap_at);
            view.position(vlines, rope).char_idx = char_idx;
        } else {
            view.position(vlines, rope).trailing_spaces -= 1;
        }
    }
}
