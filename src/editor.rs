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

    fn split_buffer(&mut self, mut start: VLineKey) {
        let mut line = &self.vlines[start];
        loop {
            if !line.continuation {
                break;
            }
            start = line.prev;
            line = &self.vlines[line.prev];
        }
        let buffer = &mut self.buffers[line.buffer_key];
        let end = buffer.end;
        if buffer.start == start {
            return;
        }
        buffer.end = start;
        let rope = &mut self.ropes[line.buffer_key];
        let char_idx = rope.byte_to_char(line.start_byte);
        let mut new_rope = rope.split_off(char_idx);
        let indent = new_rope
            .lines()
            .filter(|line| line.len_chars() > 1)
            .map(|line| line.chars().take_while(|c| *c == ' ').count())
            .min()
            .unwrap_or(0);
        let mut wrap_at = buffer.wrap_at.saturating_sub(indent);
        if wrap_at <= MIN_WRAP_AT {
            wrap_at = buffer.wrap_at;
        }
        // TODO does not dedent the buffer on the top... should we?
        if indent != buffer.indent {
            new_rope = new_rope
                .lines()
                .map(|slice| {
                    if slice.len_chars() > indent {
                        slice.slice(indent..)
                    } else {
                        slice
                    }
                })
                .flat_map(|slice| slice.chunks())
                .collect();
        }
        let new_rope_key = self.ropes.insert(new_rope);
        dbg!(indent, wrap_at);
        let new_buffer = Buffer::new(new_rope_key, start, end, wrap_at, indent + buffer.indent);
        self.buffers.insert(new_rope_key, new_buffer);
        self.vlines.update_rope(start, new_rope_key, indent);
    }

    pub fn create_block(&mut self) {
        let cursor = self.window.cursor();
        let line = &self.vlines[cursor];
        let slice = line.slice(&self.ropes);
        let indent = slice.chars().take_while(|c| *c == ' ').count();
        if indent == 0 {
            return;
        }
        let indent = slice.slice(..indent);
        let buffer = &self.buffers[line.buffer_key];
        let it = self.vlines.iter(cursor, buffer.start..buffer.end);
        let (end_key, _) = self
            .find_block_edge(it.clone(), '}', indent)
            .unwrap_or((buffer.end, &self.vlines[buffer.end]));
        let Some((_, start_line)) = self.find_block_edge(it.reversed(), '{', indent) else {
            return;
        };
        let start_key = start_line.next;
        self.split_buffer(end_key);
        self.split_buffer(start_key);
    }

    fn find_block_edge<'r, R>(
        &self,
        it: VLineIter<'r, R>,
        delimiter: char,
        indent: RopeSlice,
    ) -> Option<(VLineKey, &'r VLine)>
    where
        R: std::ops::RangeBounds<VLineKey>,
    {
        let mut res = None;
        let mut found_delimiter = false;
        for (key, line) in it {
            let slice = line.slice(&self.ropes);
            if slice.len_chars() == 0 {
                continue;
            }
            if slice.chars().any(|c| c == delimiter) {
                found_delimiter |= true;
                res = Some((key, line));
            }
            if line.continuation {
                continue;
            }
            if slice.len_chars() < indent.len_chars() || slice.slice(..indent.len_chars()) != indent
            {
                break;
            }
        }
        res.filter(|_| found_delimiter)
    }

    pub fn create_window(&mut self) {
        let line = &self.vlines[self.window.cursor()];
        let buffer = &self.buffers[line.buffer_key];
        self.window = Window::new(&self.buffers, &self.vlines, buffer.start, buffer.end);
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
}
