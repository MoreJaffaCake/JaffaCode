mod buffer;
#[cfg(feature = "crossterm")]
mod crossterm;
mod view;
mod vlines;

use buffer::*;
use ropey::*;
use vlines::*;

#[derive(derive_more::Debug)]
pub struct Editor {
    rope: Rope,
    vlines: VLines,
    buffer: Buffer,
}

impl Editor {
    pub fn new(initial_text: &str) -> Self {
        let mut rope = Rope::from_str(initial_text);
        let len_chars = rope.len_chars();
        if rope.char(len_chars - 1) != '\n' {
            rope.insert_char(len_chars, '\n');
        }
        let vlines = VLines::new(&rope, 40);
        let buffer = Buffer::new(&vlines, 40);
        Self {
            rope,
            vlines,
            buffer,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert_char(&mut self.vlines, &mut self.rope, c)
    }

    pub fn delete_char_forward(&mut self) {
        self.buffer
            .delete_char_forward(&mut self.vlines, &mut self.rope)
    }

    pub fn delete_char_backward(&mut self) {
        self.buffer
            .delete_char_backward(&mut self.vlines, &mut self.rope)
    }

    pub fn move_cursor_up(&mut self) {
        self.buffer.move_cursor_up(&self.vlines)
    }

    pub fn move_cursor_down(&mut self) {
        self.buffer.move_cursor_down(&self.vlines)
    }

    pub fn move_cursor_left(&mut self) {
        self.buffer.move_cursor_left(&self.vlines, &self.rope)
    }

    pub fn move_cursor_right(&mut self) {
        self.buffer.move_cursor_right(&self.vlines)
    }

    pub fn move_cursor_at_0(&mut self) {
        self.buffer.move_cursor_at_0()
    }

    pub fn move_cursor_at_start(&mut self) {
        self.buffer.move_cursor_at_start(&self.vlines, &self.rope)
    }

    pub fn move_cursor_at_end(&mut self) {
        self.buffer.move_cursor_at_end(&self.vlines, &self.rope)
    }

    pub fn get_display_lines(&self) -> impl Iterator<Item = RopeSlice> {
        self.buffer.get_display_lines(&self.vlines, &self.rope)
    }

    pub fn cursor_position<T: From<u16>>(&self) -> (T, T) {
        self.buffer.cursor_position()
    }

    pub fn scroll_up(&mut self) {
        self.buffer.scroll_up(&self.vlines)
    }

    pub fn scroll_down(&mut self) {
        self.buffer.scroll_down(&self.vlines)
    }
}
