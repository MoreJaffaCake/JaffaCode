// TODO
// - clipboard history: copy and cutting push to a stack, pasting pop from that stack
// - opening without argument opens the whole project with a view of everything simplified
// - dd on a line that has an indented block right after should dedent that block
// - consider splitting blocks on vertical space

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

#[derive(derive_more::Debug)]
pub struct DisplayLine<'r> {
    pub slice: RopeSlice<'r>,
    pub indent: &'static str,
    pub continuation: bool,
}

#[derive(derive_more::Debug)]
pub struct Location<'r> {
    // TODO probably should avoid unnecessary heap allocation
    pub lines: Vec<DisplayLine<'r>>,
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
    pub fn insert_char(&mut self, c: char) -> bool {
        self.create_block_at_cursor();
        if self.window.insert_char(
            &mut self.vlines,
            &mut self.ropes,
            &self.buffers,
            c,
            self.pane_height - 1,
        ) {
            return true;
        } else if c == ' ' {
            if self.indent() {
                // TODO: should the cursor be moved or not?
                /*
                for _ in 0..INDENT {
                    self.window.move_cursor_right_saturating();
                }
                */
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn delete_char_forward(&mut self) -> bool {
        self.create_block_at_cursor();
        if self
            .window
            .delete_char_forward(&mut self.vlines, &mut self.ropes, &self.buffers)
        {
            return true;
        }
        self.dedent()
    }

    #[inline]
    pub fn delete_char_backward(&mut self) -> bool {
        self.create_block_at_cursor();
        if self
            .window
            .delete_char_backward(&mut self.vlines, &mut self.ropes, &self.buffers)
        {
            return true;
        }
        if self.dedent() {
            // TODO: should the cursor be moved or not?
            /*
            for _ in 0..INDENT {
                self.window.move_cursor_left_saturating();
            }
            */
            return true;
        }
        false
    }

    #[inline]
    pub fn move_cursor_up(&mut self) -> bool {
        self.window.move_cursor_up(&self.vlines)
    }

    #[inline]
    pub fn move_cursor_down(&mut self) -> bool {
        self.window
            .move_cursor_down(&self.vlines, self.pane_height - 1)
    }

    #[inline]
    pub fn move_cursor_left(&mut self) -> bool {
        self.window
            .move_cursor_left(&self.vlines, &self.ropes, &self.buffers)
    }

    #[inline]
    pub fn move_cursor_right(&mut self) -> bool {
        self.window
            .move_cursor_right(&self.vlines, &self.ropes, &self.buffers)
    }

    #[inline]
    pub fn move_cursor_at_0(&mut self) -> bool {
        self.window.move_cursor_at_0()
    }

    #[inline]
    pub fn move_cursor_at_start(&mut self) -> bool {
        self.window
            .move_cursor_at_start(&self.vlines, &self.ropes, &self.buffers)
    }

    #[inline]
    pub fn move_cursor_at_end(&mut self) -> bool {
        self.window
            .move_cursor_at_end(&self.vlines, &self.ropes, &self.buffers)
    }

    #[inline]
    pub fn get_display_lines(&self) -> impl Iterator<Item = DisplayLine<'_>> {
        self.window
            .get_display_lines(&self.vlines, &self.ropes, &self.buffers)
            .take(self.pane_height as _)
    }

    #[inline]
    pub fn cursor_position<T: From<u16>>(&self) -> (T, T) {
        self.window.cursor_position()
    }

    #[inline]
    pub fn scroll_up(&mut self) -> bool {
        self.window.scroll_up(&self.vlines)
    }

    #[inline]
    pub fn scroll_down(&mut self) -> bool {
        self.window.scroll_down(&self.vlines)
    }

    pub fn page_up(&mut self) -> bool {
        let mut changed = false;
        for _ in 0..self.pane_height {
            if self.window.scroll_up(&self.vlines) {
                changed = true;
            } else {
                break;
            }
        }
        changed
    }

    pub fn page_down(&mut self) -> bool {
        let mut changed = false;
        for _ in 0..self.pane_height {
            if self.window.scroll_down(&self.vlines) {
                changed = true;
            } else {
                break;
            }
        }
        changed
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

    fn create_block_at_cursor(&mut self) -> Option<(BufferKey, usize)> {
        let cursor = self.window.cursor(&self.vlines);
        let key = self.create_block(cursor, cursor.detect_indent(&self.vlines, &self.ropes)?);
        Some((key, self.buffers[key].indent))
    }

    fn indent(&mut self) -> bool {
        let Some((origin, indent)) = self.create_block_at_cursor() else {
            return false;
        };
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
        let Some((origin, indent)) = self.create_block_at_cursor() else {
            return false;
        };
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

    fn create_window(&mut self, offset: usize) -> bool {
        let Some((key, indent)) = self.create_block_at_cursor() else {
            return false;
        };
        let Some(indent) = indent.checked_sub(offset * INDENT) else {
            return false;
        };
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
        true
    }

    pub fn set_window_to_cursor(&mut self) -> bool {
        self.create_window(0)
    }

    pub fn set_window_to_parent(&mut self) -> bool {
        self.create_window(1)
    }

    pub fn root_window(&mut self) -> bool {
        let line = &self.vlines[self.vlines.first()];
        let buffer = &self.buffers[line.buffer_key];
        self.window = Window::new(
            &self.buffers,
            &self.vlines,
            buffer.start,
            VLineCursor::null(),
        );
        true
    }

    pub fn update_pane_size(&mut self, width: u16, height: u16) {
        self.pane_width = width;
        self.pane_height = height;
    }

    pub fn location(&mut self) -> Location<'_> {
        // TODO should this use self.start instead?
        let start = self.window.start();
        let buffer_key = self.create_block(
            start,
            start.detect_indent(&self.vlines, &self.ropes).unwrap_or(0),
        );
        let mut start = self.buffers[buffer_key].start;
        if !start.move_prev_logical(&self.vlines) {
            Location {
                lines: Default::default(),
            }
        } else {
            let buffer_key = self.create_block(
                start,
                start.detect_indent(&self.vlines, &self.ropes).unwrap_or(0),
            );
            let buffer = &self.buffers[buffer_key];
            let end = buffer.end.head_key();
            let dedent = buffer.indent;
            if let Some(prev) = start.peek_prev_logical(&self.vlines) {
                let prev_buffer_key = self.create_block(
                    prev,
                    prev.detect_indent(&self.vlines, &self.ropes).unwrap_or(0),
                );
                if prev_buffer_key != buffer_key {
                    if start.full_slice(&self.vlines, &self.ropes).chars().next() != Some('}') {
                        while let Some(prev) = start.peek_prev_logical(&self.vlines) {
                            let prev_buffer_key = self.create_block(
                                prev,
                                prev.detect_indent(&self.vlines, &self.ropes).unwrap_or(0),
                            );
                            if prev.full_slice(&self.vlines, &self.ropes).chars().next()
                                == Some('}')
                            {
                                break;
                            }
                            let prev_buffer = &self.buffers[prev_buffer_key];
                            if prev_buffer.indent < dedent {
                                break;
                            }
                            start = prev;
                            if prev_buffer.indent == dedent {
                                break;
                            }
                        }
                    }
                }
            }
            Location {
                lines: DisplayLineIter {
                    ropes: &self.ropes,
                    buffers: &self.buffers,
                    vlines_iter: self.vlines.iter(start.key(&self.vlines)),
                    end,
                    dedent,
                    prepend_newlines: 0,
                    empty_slice: self.vlines[start].slice(&self.ropes).slice(0..0),
                }
                .collect(),
            }
        }
    }
}

#[derive(derive_more::Debug)]
pub struct DisplayLineIter<'v, 'r, 'b> {
    #[debug(skip)]
    pub ropes: &'r RopeMap,
    #[debug(skip)]
    pub buffers: &'b BufferMap,
    #[debug(skip)]
    pub vlines_iter: VLineIter<'v>,
    pub end: VLineKey,
    pub dedent: usize,
    pub prepend_newlines: usize,
    #[debug(skip)]
    pub empty_slice: RopeSlice<'r>,
}

impl<'v, 'r, 'b> Iterator for DisplayLineIter<'v, 'r, 'b> {
    type Item = DisplayLine<'r>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.prepend_newlines > 0 {
            self.prepend_newlines -= 1;
            return Some(DisplayLine {
                slice: self.empty_slice.clone(),
                indent: &"",
                continuation: false,
            });
        }
        let (key, line) = self.vlines_iter.next()?;
        if key == self.end {
            return None;
        }
        debug_assert!(self.buffers[line.buffer_key].indent >= self.dedent);
        let indent =
            self.buffers[line.buffer_key].indent - self.dedent + line.continuation.unwrap_or(0);
        let slice = line.slice(&self.ropes);
        debug_assert!(
            slice
                .chars_at(slice.len_chars())
                .reversed()
                .skip(1)
                .all(|c| c != '\n'),
            "newline in DisplayLine: {:?}",
            slice
        );
        Some(DisplayLine {
            slice,
            indent: &HSPACES[..indent],
            continuation: line.is_continuation(),
        })
    }
}
