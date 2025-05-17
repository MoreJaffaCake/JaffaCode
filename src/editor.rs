mod buffer;
#[cfg(feature = "crossterm")]
mod crossterm;
mod vlines;
mod window;

use self::buffer::*;
use self::vlines::*;
use self::window::*;

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
    window: Window,
    pane_width: u16,
    pane_height: u16,
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
        buffers.insert(buffer_key, Buffer::new(vlines.first(), vlines.last(), 40));

        let window = Window::new(vlines.first(), vlines.empty());

        Self {
            ropes,
            vlines,
            buffers,
            active_buffer: buffer_key,
            window,
            pane_width: 0,
            pane_height: 0,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let buffer = &mut self.buffers[self.active_buffer];
        buffer.insert_char(&mut self.vlines, &mut self.ropes, &mut self.window, c)
    }

    pub fn delete_char_forward(&mut self) {
        let buffer = &mut self.buffers[self.active_buffer];
        buffer.delete_char_forward(&mut self.vlines, &mut self.ropes, &mut self.window)
    }

    pub fn delete_char_backward(&mut self) {
        let buffer = &mut self.buffers[self.active_buffer];
        buffer.delete_char_backward(&mut self.vlines, &mut self.ropes, &mut self.window)
    }

    pub fn move_cursor_up(&mut self) {
        self.window.move_cursor_up(&self.vlines)
    }

    pub fn move_cursor_down(&mut self) {
        if self.window.cur_y < self.pane_height - 1 {
            self.window.move_cursor_down(&self.vlines)
        }
    }

    pub fn move_cursor_left(&mut self) {
        self.window.move_cursor_left(&self.vlines, &self.ropes)
    }

    pub fn move_cursor_right(&mut self) {
        self.window.move_cursor_right(&self.vlines, &self.buffers)
    }

    pub fn move_cursor_at_0(&mut self) {
        self.window.move_cursor_at_0()
    }

    pub fn move_cursor_at_start(&mut self) {
        self.window.move_cursor_at_start(&self.vlines, &self.ropes)
    }

    pub fn move_cursor_at_end(&mut self) {
        self.window.move_cursor_at_end(&self.vlines, &self.ropes)
    }

    pub fn get_display_lines(&self) -> impl Iterator<Item = RopeSlice> {
        self.window
            .get_display_lines(&self.vlines, &self.ropes)
            .take(self.pane_height as _)
    }

    pub fn cursor_position<T: From<u16>>(&self) -> (T, T) {
        self.window.cursor_position()
    }

    pub fn scroll_up(&mut self, amount: usize) {
        for _ in 0..amount {
            if !self.window.scroll_up(&self.vlines) {
                break;
            }
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        for _ in 0..amount {
            if !self.window.scroll_down(&self.vlines) {
                break;
            }
        }
    }

    pub fn page_up(&mut self) {
        for _ in 0..self.pane_height {
            if !self.window.scroll_up(&self.vlines) {
                break;
            }
        }
    }

    pub fn page_down(&mut self) {
        for _ in 0..self.pane_height {
            if !self.window.scroll_down(&self.vlines) {
                break;
            }
        }
    }

    pub fn split_buffer(&mut self) {
        let cursor = self.window.cursor;
        let line = &self.vlines[cursor];
        self.buffers[line.buffer_key].end = cursor;
        let rope = &mut self.ropes[line.buffer_key];
        let char_idx = rope.byte_to_char(line.start_byte);
        let new_rope = rope.split_off(char_idx);
        let new_buffer_key = self.ropes.insert(new_rope);
        let new_buffer = Buffer::new(cursor, self.vlines.empty(), 40);
        self.buffers.insert(new_buffer_key, new_buffer);
        self.vlines.update_rope(cursor, new_buffer_key);
    }

    pub fn create_window(&mut self) {
        let line = &self.vlines[self.window.cursor];
        let buffer = &self.buffers[line.buffer_key];
        self.window = Window::new(buffer.start, buffer.end);
    }

    pub fn root_window(&mut self) {
        let line = &self.vlines[self.vlines.first()];
        let buffer = &self.buffers[line.buffer_key];
        self.window = Window::new(buffer.start, self.vlines.empty());
    }

    pub fn update_pane_size(&mut self, width: u16, height: u16) {
        self.pane_width = width;
        self.pane_height = height;
    }
}
