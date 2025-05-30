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
    pub fn new(ropes: &RopeMap, buffer_key: BufferKey, wrap_at: usize) -> Self {
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
            let key = instance.wrap(ropes, key, wrap_at);
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
            let key = instance.wrap(ropes, key, wrap_at);
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
    pub fn iter<R>(&self, index: VLineKey, range: R) -> VLineIter<R>
    where
        R: std::ops::RangeBounds<VLineKey>,
    {
        VLineIter {
            arena: &self.arena,
            index,
            range,
            reversed: false,
        }
    }

    pub fn insert(
        &mut self,
        ropes: &RopeMap,
        at: VLineKey,
        bytes: usize,
        wrap_at: usize,
    ) -> VLineKey {
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

    pub fn remove(&mut self, ropes: &RopeMap, at: VLineKey, bytes: usize, wrap_at: usize) {
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
    pub fn contains_key(&self, key: VLineKey) -> bool {
        self.arena.contains_key(key)
    }

    #[inline]
    pub fn get(&self, key: VLineKey) -> Option<&VLine> {
        self.arena.get(key)
    }

    pub fn update_rope(
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

    pub fn full_slice<'r>(&self, ropes: &'r RopeMap, key: VLineKey) -> RopeSlice<'r> {
        let (_, end_line) = self
            .iter(key, ..)
            .skip(1)
            .take_while(|(_, line)| line.continuation)
            .last()
            .unwrap_or((key, &self.arena[key]));
        let (_, start_line) = self
            .iter(key, ..)
            .reversed()
            .find(|(_, line)| !line.continuation)
            .unwrap();
        debug_assert_eq!(start_line.buffer_key, end_line.buffer_key);
        ropes[start_line.buffer_key].byte_slice(start_line.start_byte..end_line.end_byte)
    }

    pub fn detect_indent(&self, ropes: &RopeMap, key: VLineKey) -> Option<usize> {
        let slice = self.full_slice(ropes, key);
        let indent = slice.chars().take_while(|c| *c == ' ').count();
        (indent < slice.len_chars() - 1).then_some(indent)
    }
}

impl std::ops::Index<VLineKey> for VLines {
    type Output = VLine;

    fn index(&self, key: VLineKey) -> &Self::Output {
        &self.arena[key]
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
}

#[derive(derive_more::Debug, Clone)]
pub struct VLineIter<'v, R> {
    #[debug(skip)]
    arena: &'v SlotMap<VLineKey, VLine>,
    #[debug(skip)]
    index: VLineKey,
    #[debug(skip)]
    range: R,
    reversed: bool,
}

impl<'v, R> Iterator for VLineIter<'v, R>
where
    R: std::ops::RangeBounds<VLineKey>,
{
    type Item = (VLineKey, &'v VLine);

    fn next(&mut self) -> Option<Self::Item> {
        use std::ops::Bound::*;
        let key = self.index;
        let line = self.arena.get(key)?;
        if self.reversed {
            match self.range.start_bound() {
                Excluded(b) if *b == key => return None,
                Included(b) if *b == line.next => return None,
                _ => self.index = line.prev,
            }
        } else {
            match self.range.end_bound() {
                Excluded(b) if *b == key => return None,
                Included(b) if *b == line.prev => return None,
                _ => self.index = line.next,
            }
        }
        Some((key, line))
    }
}

impl<'v, R> VLineIter<'v, R>
where
    R: std::ops::RangeBounds<VLineKey> + Clone,
{
    pub fn reversed(&self) -> Self {
        let mut instance = self.clone();
        instance.reversed = true;
        instance
    }
}
