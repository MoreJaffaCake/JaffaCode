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

const MAX_WRAP_AT: usize = 200;
const MIN_WRAP_AT: usize = 12;
static HSPACES: &str = "                                                                                                                                                                                                        ";
static VSPACES: &str = "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n";

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
    window: Window,
    pane_width: u16,
    pane_height: u16,
}

#[derive(derive_more::Debug, Clone)]
pub struct DisplayLine<'r> {
    pub slice: RopeSlice<'r>,
    pub indent: &'static str,
    pub continuation: bool,
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

        let vlines = VLines::new(&ropes, rope_key, 40);

        let mut buffers = BufferMap::new();
        buffers.insert(
            rope_key,
            Buffer::new(rope_key, vlines.first(), vlines.last(), 40, 0),
        );

        let window = Window::new(&buffers, &vlines, vlines.first(), VLineKey::null());

        Self {
            ropes,
            vlines,
            buffers,
            window,
            pane_width: 0,
            pane_height: 0,
        }
    }

    #[inline]
    pub fn insert_char(&mut self, c: char) {
        self.window.insert_char(
            &mut self.vlines,
            &mut self.ropes,
            &self.buffers,
            c,
            self.pane_height - 1,
        )
    }

    #[inline]
    pub fn delete_char_forward(&mut self) {
        self.window
            .delete_char_forward(&mut self.vlines, &mut self.ropes, &self.buffers)
    }

    #[inline]
    pub fn delete_char_backward(&mut self) {
        self.window
            .delete_char_backward(&mut self.vlines, &mut self.ropes, &self.buffers)
    }

    #[inline]
    pub fn move_cursor_up(&mut self) {
        self.window.move_cursor_up(&self.vlines)
    }

    #[inline]
    pub fn move_cursor_down(&mut self) {
        self.window
            .move_cursor_down(&self.vlines, self.pane_height - 1)
    }

    #[inline]
    pub fn move_cursor_left(&mut self) {
        self.window.move_cursor_left(&self.vlines, &self.ropes)
    }

    #[inline]
    pub fn move_cursor_right(&mut self) {
        self.window.move_cursor_right(&self.vlines, &self.buffers)
    }

    #[inline]
    pub fn move_cursor_at_0(&mut self) {
        self.window.move_cursor_at_0()
    }

    #[inline]
    pub fn move_cursor_at_start(&mut self) {
        self.window
            .move_cursor_at_start(&self.vlines, &self.ropes, &self.buffers)
    }

    #[inline]
    pub fn move_cursor_at_end(&mut self) {
        self.window
            .move_cursor_at_end(&self.vlines, &self.ropes, &self.buffers)
    }

    #[inline]
    pub fn get_display_lines(&self) -> impl Iterator<Item = DisplayLine> {
        self.window
            .get_display_lines(&self.vlines, &self.ropes, &self.buffers)
            .take(self.pane_height as _)
    }

    #[inline]
    pub fn cursor_position<T: From<u16>>(&self) -> (T, T) {
        self.window.cursor_position()
    }

    #[inline]
    pub fn scroll_up(&mut self) {
        self.window.scroll_up(&self.vlines);
    }

    #[inline]
    pub fn scroll_down(&mut self) {
        self.window.scroll_down(&self.vlines);
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

    fn split_buffer(&mut self, mut at: VLineKey, indent: usize) {
        let mut line = &self.vlines[at];
        loop {
            if !line.continuation {
                break;
            }
            at = line.prev;
            line = &self.vlines[line.prev];
        }
        let buffer_key = line.buffer_key;
        let buffer = &mut self.buffers[buffer_key];
        let end = buffer.end;
        let mut new_rope = if buffer.start == at {
            self.ropes[buffer_key].clone()
        } else {
            buffer.end = at;
            let rope = &mut self.ropes[buffer_key];
            let char_idx = rope.byte_to_char(line.start_byte);
            rope.split_off(char_idx)
        };
        let mut wrap_at = buffer.wrap_at.saturating_sub(indent);
        if wrap_at <= MIN_WRAP_AT {
            wrap_at = buffer.wrap_at;
        }
        if indent > 0 {
            new_rope = new_rope
                .lines()
                .map(|slice| {
                    if slice.len_chars() > 1 {
                        debug_assert!(slice.len_chars() > indent, "{slice:?} (indent: {indent})");
                        slice.slice(indent..)
                    } else {
                        slice
                    }
                })
                .flat_map(|slice| slice.chunks())
                .collect();
        }
        let new_rope_key = self.ropes.insert(new_rope);
        let new_buffer = Buffer::new(new_rope_key, at, end, wrap_at, buffer.indent + indent);
        self.buffers.insert(new_rope_key, new_buffer);
        self.vlines.update_rope(at, new_rope_key, indent);
    }

    pub fn create_block(&mut self) {
        let cursor = self.window.cursor();
        let buffer_key = self.vlines[cursor].buffer_key;
        let slice = self.vlines.full_slice(&self.ropes, cursor);
        let indent = slice.chars().take_while(|c| *c == ' ').count();
        if indent == 0 {
            return;
        }
        let indent = slice.slice(..indent);
        let buffer = &self.buffers[buffer_key];
        let buffer_start = buffer.start;
        let it = self.vlines.iter(cursor, buffer.start..buffer.end);
        let end_bound = self.find_block_edge(it.clone(), indent);
        let start_bound = self.find_block_edge(it.reversed(), indent);
        let indent = indent.len_chars();
        if let Some(bound) = end_bound {
            debug_assert_eq!(self.vlines[bound].buffer_key, buffer_key);
            self.split_buffer(bound, 0);
        }
        if let Some(bound) = start_bound {
            let next = self.vlines[bound].next;
            debug_assert_eq!(self.vlines[next].buffer_key, buffer_key);
            self.split_buffer(next, indent);
        } else {
            self.split_buffer(buffer_start, indent);
        }
    }

    fn find_block_edge<R>(&self, mut it: VLineIter<R>, indent: RopeSlice) -> Option<VLineKey>
    where
        R: std::ops::RangeBounds<VLineKey>,
    {
        it.find_map(|(key, _)| {
            let slice = self.vlines.full_slice(&self.ropes, key);
            let slice = slice.slice(..(slice.len_chars() - 1));
            if slice.len_chars() == 0 || slice.chars().all(|c| c == ' ') {
                return None;
            }
            if slice.len_chars() < indent.len_chars() || slice.slice(..indent.len_chars()) != indent
            {
                return Some(key);
            }
            None
        })
    }

    pub fn create_window(&mut self) {
        let line = &self.vlines[self.window.cursor()];
        let start_buffer = &self.buffers[line.buffer_key];
        let (_, last_buffer) = self
            .buffers(start_buffer.key)
            .take_while(|(_, buffer)| buffer.indent >= start_buffer.indent)
            .last()
            .unwrap();
        self.window = Window::new(
            &self.buffers,
            &self.vlines,
            start_buffer.start,
            last_buffer.end,
        );
    }

    pub fn root_window(&mut self) {
        let line = &self.vlines[self.vlines.first()];
        let buffer = &self.buffers[line.buffer_key];
        self.window = Window::new(&self.buffers, &self.vlines, buffer.start, VLineKey::null());
    }

    pub fn update_pane_size(&mut self, width: u16, height: u16) {
        self.pane_width = width;
        self.pane_height = height;
    }

    pub fn buffers(&self, at: BufferKey) -> BufferIter {
        BufferIter {
            vlines: &self.vlines,
            buffers: &self.buffers,
            index: at,
        }
    }
}

#[derive(derive_more::Debug, Clone)]
pub struct BufferIter<'v, 'b> {
    vlines: &'v VLines,
    buffers: &'b BufferMap,
    index: BufferKey,
}

impl<'v, 'b> Iterator for BufferIter<'v, 'b> {
    type Item = (BufferKey, &'b Buffer);

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.index;
        let buffer = &self.buffers.get(key)?;
        if let Some(next_line) = self.vlines.get(buffer.end) {
            self.index = next_line.buffer_key;
        } else {
            self.index = BufferKey::null();
        }
        Some((key, buffer))
    }
}
