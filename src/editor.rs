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
    pub struct RopeKey;
}
pub type RopeMap = SlotMap<RopeKey, Rope>;

new_key_type! {
    pub struct BufferKey;
}
pub type BufferMap = SlotMap<BufferKey, Buffer>;

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
        let rope_key = ropes.insert(rope);

        // TODO put active_buffer instead of rope_key?
        let vlines = VLines::new(&ropes[rope_key], rope_key, 40);

        let mut buffers = BufferMap::with_key();
        let active_buffer = buffers.insert(Buffer::new(rope_key, 40));

        let mut view = View::new(active_buffer, rope_key, vlines.first());
        view.scroll_down(&vlines);
        view.scroll_down(&vlines);

        Self {
            ropes,
            vlines,
            buffers,
            active_buffer,
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
}
