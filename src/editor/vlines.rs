use super::*;

new_key_type! {
    pub struct VLineKey;
}

#[derive(derive_more::Debug)]
pub struct VLines {
    #[debug(skip)]
    arena: SlotMap<VLineKey, VLine>,
    #[debug(skip)]
    first: VLineKey,
}

impl VLines {
    pub fn new(ropes: &RopeMap, buffer_key: BufferKey) -> Self {
        let arena = SlotMap::<VLineKey, VLine>::with_key();
        let mut instance = Self {
            arena,
            first: VLineKey::null(),
        };
        let rope = &ropes[buffer_key];
        let mut it = rope.lines();

        let (mut prev, mut prev_end) = {
            let line = it.next().unwrap();
            let end_byte = line.len_bytes();
            let key = instance.arena.insert(VLine {
                prev: VLineKey::null(),
                next: VLineKey::null(),
                buffer_key,
                start_byte: 0,
                end_byte,
                continuation: false,
            });
            instance.first = key;
            let key = instance.wrap(ropes, key, WRAP_AT);
            (key, end_byte)
        };

        for line in it {
            let len_bytes = line.len_bytes();
            if len_bytes == 0 {
                break;
            }
            let end_byte = prev_end + len_bytes;
            let key = instance.arena.insert(VLine {
                prev: prev,
                next: VLineKey::null(),
                buffer_key,
                start_byte: prev_end,
                end_byte,
                continuation: false,
            });
            instance.arena[prev].next = key;
            let key = instance.wrap(ropes, key, WRAP_AT);
            prev_end = end_byte;
            prev = key;
        }

        instance
    }

    fn wrap(&mut self, ropes: &RopeMap, mut key: VLineKey, wrap_at: usize) -> VLineKey {
        loop {
            let line = &self.arena[key];
            let slice = line.slice(ropes);
            let len_chars = slice.len_chars();
            if len_chars <= wrap_at + 1 {
                let newline_idx = slice
                    .chars()
                    .enumerate()
                    .find_map(|(i, c)| (c == '\n').then_some(i));
                if newline_idx == Some(len_chars.saturating_sub(1)) {
                    break;
                } else if let Some(newline_idx) = newline_idx {
                    let byte_idx = slice.char_to_byte(newline_idx + 1);
                    key = self.split_line(key, byte_idx, false);
                } else if self.arena.contains_key(line.next) {
                    debug_assert!(
                        self.arena[line.next].buffer_key == line.buffer_key,
                        "missing newline at the end of a buffer"
                    );
                    self.merge_next(key);
                } else {
                    unreachable!("missing newline at EOF");
                }
            } else {
                let byte_idx = slice.char_to_byte(wrap_at);
                key = self.split_line(key, byte_idx, true);
            }
        }
        key
    }

    #[inline(always)]
    pub fn iter(&self, index: VLineKey) -> VLineIter {
        VLineIter {
            vlines: self,
            index,
        }
    }

    fn insert(&mut self, ropes: &RopeMap, at: VLineKey, bytes: usize, wrap_at: usize) -> VLineKey {
        let mut key = at;
        let buffer_key;
        {
            let line = &mut self.arena[key];
            line.end_byte += bytes;
            key = line.next;
            buffer_key = line.buffer_key;
        }
        while let Some(line) = self.arena.get_mut(key) {
            if line.buffer_key != buffer_key {
                break;
            }
            line.start_byte += bytes;
            line.end_byte += bytes;
            key = line.next;
        }
        self.wrap(ropes, at, wrap_at)
    }

    fn remove(&mut self, ropes: &RopeMap, at: VLineKey, bytes: usize, wrap_at: usize) {
        let mut key = at;
        let buffer_key;
        {
            let line = &mut self.arena[key];
            line.end_byte -= bytes;
            buffer_key = line.buffer_key;
            key = line.next;
        }
        while let Some(line) = self.arena.get_mut(key) {
            if line.buffer_key != buffer_key {
                break;
            }
            line.start_byte -= bytes;
            line.end_byte -= bytes;
            key = line.next;
        }
        self.wrap(ropes, at, wrap_at);
    }

    #[inline]
    fn merge_next(&mut self, key: VLineKey) {
        let b = self.arena.remove(self.arena[key].next).unwrap();
        if let Some(c) = self.arena.get_mut(b.next) {
            c.prev = key;
        }
        let a = &mut self.arena[key];
        a.next = b.next;
        a.end_byte = b.end_byte;
    }

    #[inline]
    fn split_line(&mut self, key: VLineKey, byte_idx: usize, continuation: bool) -> VLineKey {
        let line = &self.arena[key];
        let split_byte = line.start_byte + byte_idx;
        let next = line.next;
        let new_line = VLine {
            prev: key,
            next,
            buffer_key: line.buffer_key,
            start_byte: split_byte,
            end_byte: line.end_byte,
            continuation,
        };
        debug_assert!(new_line.start_byte != new_line.end_byte);
        let new_key = self.arena.insert(new_line);
        if let Some(line) = self.arena.get_mut(next) {
            line.prev = new_key;
        }
        let line = &mut self.arena[key];
        line.end_byte = split_byte;
        line.next = new_key;
        new_key
    }

    #[inline(always)]
    pub fn first(&self) -> VLineKey {
        self.first
    }

    #[inline]
    pub fn get(&self, key: VLineKey) -> Option<&VLine> {
        self.arena.get(key)
    }

    fn update_rope(
        &mut self,
        mut key: VLineKey,
        new_buffer_key: BufferKey,
        indent: usize,
    ) -> VLineKey {
        let mut line = &mut self.arena[key];
        let old_buffer_key = line.buffer_key;
        let new_start_byte = line.start_byte;
        let mut cumulative_indent = 0;
        loop {
            line.buffer_key = new_buffer_key;
            line.start_byte -= new_start_byte + cumulative_indent;
            line.end_byte -= new_start_byte + cumulative_indent;
            if !line.continuation && line.end_byte - line.start_byte > indent {
                line.end_byte -= indent;
                cumulative_indent += indent;
            }
            let next = line.next;
            let Some(next_line) = self.arena.get_mut(next) else {
                break;
            };
            if next_line.buffer_key != old_buffer_key {
                break;
            }
            key = next;
            line = next_line;
        }
        key
    }
}

impl std::ops::Index<VLineKey> for VLines {
    type Output = VLine;

    fn index(&self, key: VLineKey) -> &Self::Output {
        &self.arena[key]
    }
}

impl std::ops::Index<VLineCursor> for VLines {
    type Output = VLine;

    fn index(&self, cur: VLineCursor) -> &Self::Output {
        debug_assert!(!(cur.key.is_null() && cur.offset > 0));
        let mut line = &self.arena[cur.key];
        for _ in 0..cur.offset {
            line = &self.arena[line.next];
        }
        line
    }
}

#[derive(derive_more::Debug)]
pub struct VLine {
    #[debug(skip)]
    pub prev: VLineKey,
    #[debug(skip)]
    pub next: VLineKey,
    #[debug(skip)]
    pub buffer_key: BufferKey,
    pub start_byte: usize,
    pub end_byte: usize,
    pub continuation: bool,
}

impl VLine {
    #[inline(always)]
    pub fn slice<'r>(&self, ropes: &'r RopeMap) -> RopeSlice<'r> {
        ropes[self.buffer_key].byte_slice(self.start_byte..self.end_byte)
    }

    #[inline]
    pub fn is_indented_at(&self, ropes: &RopeMap, at: usize) -> bool {
        self.slice(ropes)
            .chars()
            // NOTE: **round up** with INDENT
            .take((at + INDENT - 1) / INDENT * INDENT)
            .all(|c| c == ' ')
    }
}

#[derive(derive_more::Debug, Clone)]
pub struct VLineIter<'v> {
    #[debug(skip)]
    vlines: &'v VLines,
    #[debug(skip)]
    index: VLineKey,
}

impl<'v> Iterator for VLineIter<'v> {
    type Item = (VLineKey, &'v VLine);

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.index;
        let line = self.vlines.get(key)?;
        self.index = line.next;
        Some((key, line))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VLineCursor {
    key: VLineKey,
    offset: usize,
}

impl VLineCursor {
    #[track_caller]
    pub fn new(vlines: &VLines, key: VLineKey) -> Self {
        debug_assert!(!vlines[key].continuation);
        Self { key, offset: 0 }
    }

    #[inline]
    #[track_caller]
    pub fn key(&self, vlines: &VLines) -> VLineKey {
        debug_assert!(!(self.key.is_null() && self.offset > 0));
        let mut key = self.key;
        for _ in 0..self.offset {
            key = vlines[key].next;
        }
        key
    }

    #[inline(always)]
    pub fn head_key(&self) -> VLineKey {
        self.key
    }

    #[inline(always)]
    pub fn line<'v>(&self, vlines: &'v VLines) -> Option<&'v VLine> {
        vlines.get(self.key(vlines))
    }

    #[inline(always)]
    pub fn head_line<'v>(&self, vlines: &'v VLines) -> Option<&'v VLine> {
        vlines.get(self.head_key())
    }

    #[inline(always)]
    pub fn null() -> Self {
        Self {
            key: VLineKey::null(),
            offset: 0,
        }
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.key.is_null()
    }

    fn last_vline<'v>(&self, vlines: &'v VLines) -> (VLineKey, &'v VLine) {
        vlines
            .iter(self.key)
            .skip(1)
            .take_while(|(_, line)| line.continuation)
            .last()
            .unwrap_or_else(|| (self.key, &vlines[self.key]))
    }

    pub fn peek_prev_logical(&self, vlines: &VLines) -> Option<Self> {
        let mut key = vlines.get(self.key)?.prev;
        loop {
            let line = vlines.get(key)?;
            if !line.continuation {
                break;
            }
            key = line.prev;
        }
        Some(Self { key, offset: 0 })
    }

    pub fn peek_next_logical(&self, vlines: &VLines) -> Option<Self> {
        let mut key = vlines.get(self.key)?.next;
        loop {
            let line = vlines.get(key)?;
            if !line.continuation {
                break;
            }
            key = line.next;
        }
        Some(Self { key, offset: 0 })
    }

    pub fn move_next_logical(&mut self, vlines: &VLines) -> bool {
        let Some(next) = self.peek_next_logical(vlines) else {
            return false;
        };
        *self = next;
        true
    }

    pub fn move_next_logical_if(
        &mut self,
        vlines: &VLines,
        cond: impl FnOnce(VLineCursor) -> bool,
    ) -> bool {
        let Some(next) = self.peek_next_logical(vlines) else {
            return false;
        };
        if !(cond)(next) {
            return false;
        }
        *self = next;
        true
    }

    pub fn peek_next_visual(&self, vlines: &VLines) -> Option<Self> {
        let next = self.line(vlines)?.next;
        let (key, offset) = if vlines.get(next)?.continuation {
            (self.key, self.offset + 1)
        } else {
            (next, 0)
        };
        Some(Self { key, offset })
    }

    pub fn move_next_visual(&mut self, vlines: &VLines) -> bool {
        let Some(next) = self.peek_next_visual(vlines) else {
            return false;
        };
        *self = next;
        true
    }

    pub fn move_next_visual_if(
        &mut self,
        vlines: &VLines,
        cond: impl FnOnce(VLineCursor) -> bool,
    ) -> bool {
        let Some(next) = self.peek_next_visual(vlines) else {
            return false;
        };
        if !(cond)(next) {
            return false;
        }
        *self = next;
        true
    }

    pub fn peek_prev_visual(&self, vlines: &VLines) -> Option<Self> {
        if self.offset > 0 {
            Some(Self {
                offset: self.offset - 1,
                ..*self
            })
        } else {
            let mut key = vlines[self.key].prev;
            if key.is_null() {
                return None;
            }
            let mut offset = 0;
            loop {
                let line = &vlines[key];
                if !line.continuation {
                    break;
                }
                offset += 1;
                key = line.prev;
            }
            Some(Self { key, offset })
        }
    }

    pub fn move_prev_visual(&mut self, vlines: &VLines) -> bool {
        let Some(prev) = self.peek_prev_visual(vlines) else {
            return false;
        };
        *self = prev;
        true
    }

    pub fn move_prev_visual_if(
        &mut self,
        vlines: &VLines,
        cond: impl FnOnce(VLineCursor) -> bool,
    ) -> bool {
        let Some(prev) = self.peek_prev_visual(vlines) else {
            return false;
        };
        if !(cond)(prev) {
            return false;
        }
        *self = prev;
        true
    }

    #[inline(always)]
    pub fn iter_logical<'v>(&self, vlines: &'v VLines) -> VLineCursorLogicalIter<'v> {
        VLineCursorLogicalIter {
            vlines,
            index: Some(*self),
            reversed: false,
            start_bound: VLineCursor::null(),
            end_bound: VLineCursor::null(),
        }
    }

    pub fn full_slice<'r>(&self, vlines: &VLines, ropes: &'r RopeMap) -> RopeSlice<'r> {
        let start_line = &vlines[self.key];
        let (_, end_line) = self.last_vline(vlines);
        debug_assert_eq!(start_line.buffer_key, end_line.buffer_key);
        ropes[start_line.buffer_key].byte_slice(start_line.start_byte..end_line.end_byte)
    }

    pub fn detect_indent(&self, vlines: &VLines, ropes: &RopeMap) -> Option<usize> {
        let slice = self.full_slice(vlines, ropes);
        let indent = slice.chars().take_while(|c| *c == ' ').count();
        (indent < slice.len_chars() - 1).then_some(indent / INDENT * INDENT)
    }

    #[inline]
    pub fn insert(
        &self,
        vlines: &mut VLines,
        ropes: &RopeMap,
        bytes: usize,
        wrap_at: usize,
    ) -> VLineKey {
        vlines.insert(ropes, self.key(vlines), bytes, wrap_at)
    }

    #[inline]
    pub fn remove(&self, vlines: &mut VLines, ropes: &RopeMap, bytes: usize, wrap_at: usize) {
        vlines.remove(ropes, self.key(vlines), bytes, wrap_at);
    }

    pub fn rewrap(&self, vlines: &mut VLines, ropes: &RopeMap, wrap_at: usize) -> Self {
        Self {
            key: vlines.wrap(ropes, self.key, wrap_at),
            offset: 0,
        }
    }

    pub fn update_rope(
        &self,
        vlines: &mut VLines,
        new_buffer_key: BufferKey,
        indent: usize,
    ) -> Self {
        let key = vlines.update_rope(self.key, new_buffer_key, indent);
        Self { key, offset: 0 }
    }
}

#[derive(derive_more::Debug, Clone)]
pub struct VLineCursorLogicalIter<'v> {
    #[debug(skip)]
    vlines: &'v VLines,
    #[debug(skip)]
    index: Option<VLineCursor>,
    reversed: bool,
    start_bound: VLineCursor,
    end_bound: VLineCursor,
}

impl Iterator for VLineCursorLogicalIter<'_> {
    type Item = VLineCursor;

    fn next(&mut self) -> Option<Self::Item> {
        let current_index = self.index?;
        self.index = if self.reversed {
            if current_index != self.start_bound {
                self.index?.peek_prev_logical(&self.vlines)
            } else {
                None
            }
        } else {
            self.index?
                .peek_next_logical(&self.vlines)
                .filter(|idx| *idx != self.end_bound)
        };
        Some(current_index)
    }
}

impl VLineCursorLogicalIter<'_> {
    pub fn reversed(mut self) -> Self {
        self.reversed ^= true;
        self
    }

    pub fn start_bounded(mut self, cursor: VLineCursor) -> Self {
        self.start_bound = cursor;
        self
    }

    pub fn end_bounded(mut self, cursor: VLineCursor) -> Self {
        self.end_bound = cursor;
        self
    }

    pub fn find_block_edge(&mut self, ropes: &RopeMap, indent: usize) -> Option<VLineCursor> {
        let vlines = &self.vlines;
        let indent = &HSPACES[..indent];
        self.find_map(|cur| {
            let slice = cur.full_slice(vlines, ropes);
            let slice = slice.slice(..(slice.len_chars() - 1));
            if slice.len_chars() == 0 || slice.chars().all(|c| c == ' ') {
                return None;
            }
            if slice.len_chars() < indent.len() || slice.slice(..indent.len()) != indent {
                return Some(cur);
            }
            None
        })
    }
}
