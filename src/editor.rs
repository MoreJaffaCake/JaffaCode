// TODO
// - clipboard history: copy and cutting push to a stack, pasting pop from that stack
// - opening without argument opens the whole project with a view of everything simplified

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

const INDENT: usize = 4;
const WRAP_AT: usize = 40;
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
    #[allow(dead_code)]
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

        let vlines = VLines::new(&ropes, rope_key);

        let mut buffers = BufferMap::new();
        buffers.insert(
            rope_key,
            Buffer::new(
                rope_key,
                VLineCursor::new(&vlines, vlines.first()),
                VLineCursor::null(),
                WRAP_AT,
                0,
            ),
        );

        let window = Window::new(
            &buffers,
            &vlines,
            VLineCursor::new(&vlines, vlines.first()),
            VLineCursor::null(),
        );

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
        if self.window.insert_char(
            &mut self.vlines,
            &mut self.ropes,
            &self.buffers,
            c,
            self.pane_height - 1,
        ) {
            return;
        } else if c == ' ' {
            if self.indent() {
                for _ in 0..INDENT {
                    self.window.move_cursor_right(&self.vlines, &self.buffers);
                }
            }
        }
    }

    #[inline]
    pub fn delete_char_forward(&mut self) {
        if self
            .window
            .delete_char_forward(&mut self.vlines, &mut self.ropes, &self.buffers)
        {
            return;
        }
        self.dedent();
    }

    #[inline]
    pub fn delete_char_backward(&mut self) {
        if self
            .window
            .delete_char_backward(&mut self.vlines, &mut self.ropes, &self.buffers)
        {
            return;
        }
        if self.dedent() {
            for _ in 0..INDENT {
                self.window.move_cursor_left_saturating();
            }
        }
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
        self.window
            .move_cursor_left(&self.vlines, &self.ropes, &self.buffers)
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

    fn split_buffer(&mut self, at: VLineCursor, indent: usize) -> BufferKey {
        let line = &self.vlines[at.head_key()];
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
                        debug_assert!(
                            slice.len_chars() > indent,
                            "dedent failed: {slice:?} (indent: {indent})",
                        );
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
        at.update_rope(&mut self.vlines, new_rope_key, indent);
        new_rope_key
    }

    fn create_block(&mut self, at: VLineCursor, indent: usize) -> BufferKey {
        let buffer_key = self.vlines[at].buffer_key;
        if indent == 0 {
            return buffer_key;
        }
        let buffer = &self.buffers[buffer_key];
        let buffer_start = buffer.start;
        let mut it = at
            .iter_logical(&self.vlines)
            .start_bounded(buffer.start)
            .end_bounded(buffer.end);
        let start_bound = it.clone().reversed().find_block_edge(&self.ropes, indent);
        let end_bound = it.find_block_edge(&self.ropes, indent);
        if let Some(bound) = end_bound {
            debug_assert_eq!(self.vlines[bound].buffer_key, buffer_key);
            self.split_buffer(bound, 0);
        }
        if let Some(bound) = start_bound {
            let next = bound.peek_next_logical(&self.vlines).unwrap();
            dbg!(
                self.vlines[next].slice(&self.ropes),
                indent,
                end_bound.is_some()
            );
            debug_assert_eq!(self.vlines[next].buffer_key, buffer_key);
            self.split_buffer(next, indent)
        } else {
            dbg!(indent, end_bound.is_some());
            self.split_buffer(buffer_start, indent)
        }
    }

    fn indent(&mut self) -> bool {
        let cursor = self.window.cursor();
        let origin = self.create_block(
            cursor,
            cursor.detect_indent(&self.vlines, &self.ropes).unwrap_or(0),
        );
        let indent = self.buffers[origin].indent;
        let mut key = origin;
        loop {
            self.buffers[key].indent(&mut self.vlines, &self.ropes);
            let Some((next, relative_indent, total_indent)) =
                self.buffers[key].find_next_block(&self.vlines, &self.ropes, &self.buffers)
            else {
                break;
            };
            if total_indent < indent || (total_indent == indent && key == origin) {
                break;
            }
            key = self.create_block(next, relative_indent);
        }
        let mut key = origin;
        loop {
            let Some((next, relative_indent, total_indent)) =
                self.buffers[key].find_prev_block(&self.vlines, &self.ropes, &self.buffers)
            else {
                break;
            };
            if total_indent < indent || (total_indent == indent && key == origin) {
                break;
            }
            key = self.create_block(next, relative_indent);
            self.buffers[key].indent(&mut self.vlines, &self.ropes);
        }
        true
    }

    fn dedent(&mut self) -> bool {
        let cursor = self.window.cursor();
        let origin = self.create_block(
            cursor,
            cursor.detect_indent(&self.vlines, &self.ropes).unwrap_or(0),
        );
        if self.buffers[origin].indent < INDENT {
            return false;
        }
        let indent = self.buffers[origin].indent;
        let mut key = origin;
        loop {
            self.buffers[key].dedent(&mut self.vlines, &self.ropes);
            let Some((next, relative_indent, total_indent)) =
                self.buffers[key].find_next_block(&self.vlines, &self.ropes, &self.buffers)
            else {
                break;
            };
            if total_indent < indent || (total_indent == indent && key == origin) {
                break;
            }
            key = self.create_block(next, relative_indent);
        }
        let mut key = origin;
        loop {
            let Some((next, relative_indent, total_indent)) =
                self.buffers[key].find_prev_block(&self.vlines, &self.ropes, &self.buffers)
            else {
                break;
            };
            if total_indent < indent || (total_indent == indent && key == origin) {
                break;
            }
            key = self.create_block(next, relative_indent);
            self.buffers[key].dedent(&mut self.vlines, &self.ropes);
        }
        true
    }

    pub fn create_window(&mut self) {
        let cursor = self.window.cursor();
        let Some(relative_indent) = cursor.detect_indent(&self.vlines, &self.ropes) else {
            return;
        };
        let indent = self.buffers[self.vlines[cursor].buffer_key].indent + relative_indent;
        let key = self.create_block(cursor, relative_indent);
        let first = std::iter::successors(Some(key), |key| {
            let prev = self.buffers[*key].start.peek_prev_logical(&self.vlines)?;
            let buffer = &self.buffers[self.vlines[prev].buffer_key];
            let detected_indent = prev.detect_indent(&self.vlines, &self.ropes).unwrap_or(0);
            (buffer.indent + detected_indent >= indent)
                .then(|| self.create_block(prev, detected_indent))
        })
        .last()
        .unwrap();
        let last = std::iter::successors(Some(key), |key| {
            let next = self.buffers[*key].end;
            if next.is_null() {
                return None;
            }
            let buffer = &self.buffers[self.vlines[next].buffer_key];
            let detected_indent = next.detect_indent(&self.vlines, &self.ropes).unwrap_or(0);
            (buffer.indent + detected_indent >= indent)
                .then(|| self.create_block(next, detected_indent))
        })
        .last()
        .unwrap();
        self.window = Window::new(
            &self.buffers,
            &self.vlines,
            self.buffers[first].start,
            self.buffers[last].end,
        );
    }

    pub fn root_window(&mut self) {
        let line = &self.vlines[self.vlines.first()];
        let buffer = &self.buffers[line.buffer_key];
        self.window = Window::new(
            &self.buffers,
            &self.vlines,
            buffer.start,
            VLineCursor::null(),
        );
    }

    pub fn update_pane_size(&mut self, width: u16, height: u16) {
        self.pane_width = width;
        self.pane_height = height;
    }
}
