mod buffer;
#[cfg(feature = "crossterm")]
mod crossterm;
mod view;
mod vlines;

use self::buffer::*;
use self::view::*;
use self::vlines::*;

use ropey::*;
use slotmap::*;

new_key_type! {
    pub struct BufferKey;
}
pub type RopeMap = SlotMap<BufferKey, Rope>;

pub type BufferMap = SecondaryMap<BufferKey, Buffer>;

#[derive(derive_more::Debug)]
pub struct Editor {
    #[debug(skip)]
    ropes: RopeMap,
    vlines: VLines,
    #[debug(skip)]
    buffers: BufferMap,
    #[debug(skip)]
    active_buffer: BufferKey,
    view: View,
}

impl Editor {
    pub fn new(initial_text: &str) -> Self {
        let mut rope = Rope::from_str(initial_text);
        let len_chars = rope.len_chars();
        if rope.char(len_chars - 1) != '\n' {
            rope.insert_char(len_chars, '\n');
        }

        let mut ropes = RopeMap::with_key();
        let buffer_key = ropes.insert(rope);

        let vlines = VLines::new(&ropes, buffer_key, 40);

        let mut buffers = BufferMap::new();
        buffers.insert(buffer_key, Buffer::new(buffer_key, 40));

        let view = View::new(buffer_key, vlines.first());

        Self {
            ropes,
            vlines,
            buffers,
            active_buffer: buffer_key,
            view,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let buffer = &mut self.buffers[self.active_buffer];
        buffer.insert_char(&mut self.vlines, &mut self.ropes, &mut self.view, c)
    }

    pub fn delete_char_forward(&mut self) {
        let buffer = &mut self.buffers[self.active_buffer];
        buffer.delete_char_forward(&mut self.vlines, &mut self.ropes, &mut self.view)
    }

    pub fn delete_char_backward(&mut self) {
        let buffer = &mut self.buffers[self.active_buffer];
        buffer.delete_char_backward(&mut self.vlines, &mut self.ropes, &mut self.view)
    }

    pub fn move_cursor_up(&mut self) {
        self.view.move_cursor_up(&self.vlines)
    }

    pub fn move_cursor_down(&mut self) {
        self.view.move_cursor_down(&self.vlines)
    }

    pub fn move_cursor_left(&mut self) {
        self.view.move_cursor_left(&self.vlines, &self.ropes)
    }

    pub fn move_cursor_right(&mut self) {
        self.view.move_cursor_right(&self.vlines, &self.buffers)
    }

    pub fn move_cursor_at_0(&mut self) {
        self.view.move_cursor_at_0()
    }

    pub fn move_cursor_at_start(&mut self) {
        self.view.move_cursor_at_start(&self.vlines, &self.ropes)
    }

    pub fn move_cursor_at_end(&mut self) {
        self.view.move_cursor_at_end(&self.vlines, &self.ropes)
    }

    pub fn get_display_lines(&self) -> impl Iterator<Item = RopeSlice> {
        self.view.get_display_lines(&self.vlines, &self.ropes)
    }

    pub fn cursor_position<T: From<u16>>(&self) -> (T, T) {
        self.view.cursor_position()
    }

    pub fn scroll_up(&mut self) {
        self.view.scroll_up(&self.vlines);
    }

    pub fn scroll_down(&mut self) {
        self.view.scroll_down(&self.vlines);
    }

    pub fn split_buffer(&mut self) {
        let line = &self.vlines[self.view.cursor];
        let rope = &mut self.ropes[line.buffer_key];
        let char_idx = rope.byte_to_char(line.start_byte);
        let new_rope = rope.split_off(char_idx);
        let new_buffer_key = self.ropes.insert(new_rope);
        let new_buffer = Buffer::new(new_buffer_key, 40);
        self.buffers.insert(new_buffer_key, new_buffer);
        self.vlines.update_rope(self.view.cursor, new_buffer_key);
    }
}
