use super::*;

new_key_type! {
    pub struct VLineKey;
}

#[derive(derive_more::Debug, Default)]
pub struct VLines {
    #[debug(skip)]
    arena: SlotMap<VLineKey, VLine>,
    #[debug(skip)]
    empty: VLineKey,
    #[debug(skip)]
    first: VLineKey,
    #[debug(skip)]
    last: VLineKey,
}

impl VLines {
    pub fn new(ropes: &RopeMap, buffer_key: BufferKey, wrap_at: usize) -> Self {
        let mut arena = SlotMap::<VLineKey, VLine>::with_key();
        let empty = arena.insert_with_key(|k| VLine {
            prev: k,
            next: k,
            buffer_key,
            start_byte: 0,
            end_byte: 0,
        });
        arena.remove(empty);

        let mut instance = Self {
            arena,
            empty,
            first: empty,
            last: empty,
        };
        let rope = &ropes[buffer_key];
        let mut it = rope.lines();

        let (mut prev, mut prev_end) = {
            let line = it.next().unwrap();
            let end_byte = line.len_bytes();
            let key = instance.arena.insert(VLine {
                prev: empty,
                next: empty,
                buffer_key,
                start_byte: 0,
                end_byte,
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
                next: empty,
                buffer_key,
                start_byte: prev_end,
                end_byte,
            });
            instance.arena[prev].next = key;
            let key = instance.wrap(ropes, key, wrap_at);
            prev_end = end_byte;
            prev = key;
        }

        instance.last = prev;

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
                    key = self.split_line(key, byte_idx);
                } else if self.arena.contains_key(line.next) {
                    self.merge_next(key);
                } else {
                    unreachable!("missing newline at EOF");
                }
            } else {
                let byte_idx = slice.char_to_byte(wrap_at);
                key = self.split_line(key, byte_idx);
            }
        }
        key
    }

    #[inline(always)]
    pub fn slices<'r>(&self, window: &Window, ropes: &'r RopeMap) -> SliceIterator<'_, 'r> {
        SliceIterator {
            arena: &self.arena,
            ropes,
            index: window.start,
            end: window.end,
        }
    }

    pub fn insert(&mut self, window: &Window, bytes: usize, ropes: &RopeMap, wrap_at: usize) {
        let mut key = window.cursor;
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
        self.wrap(ropes, window.cursor, wrap_at);
    }

    pub fn remove(&mut self, window: &Window, bytes: usize, ropes: &RopeMap, wrap_at: usize) {
        let mut key = window.cursor;
        {
            let line = &mut self.arena[key];
            line.end_byte -= bytes;
            key = line.next;
        }
        while let Some(line) = self.arena.get_mut(key) {
            line.start_byte -= bytes;
            line.end_byte -= bytes;
            key = line.next;
        }
        self.wrap(ropes, window.cursor, wrap_at);
    }

    pub fn insert_newlines(&mut self, window: &mut Window, n: usize) {
        let mut prev = window.cursor;
        let line = &self.arena[prev];
        let buffer_key = line.buffer_key;
        let mut prev_end = line.end_byte;
        let next = line.next;
        for _ in 0..n {
            let key = self.arena.insert(VLine {
                prev: prev,
                next: self.empty,
                buffer_key,
                start_byte: prev_end,
                end_byte: prev_end + 1,
            });
            self.arena[prev].next = key;
            prev = key;
            prev_end += 1;
        }
        self.arena[prev].next = next;
        window.cursor = prev;
        window.cursor_idx += n;
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
    fn split_line(&mut self, key: VLineKey, byte_idx: usize) -> VLineKey {
        let line = &self.arena[key];
        let split_byte = line.start_byte + byte_idx;
        let next = line.next;
        let new_line = VLine {
            prev: key,
            next,
            buffer_key: line.buffer_key,
            start_byte: split_byte,
            end_byte: line.end_byte,
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
    pub fn empty(&self) -> VLineKey {
        self.empty
    }

    #[inline(always)]
    pub fn first(&self) -> VLineKey {
        self.first
    }

    #[inline(always)]
    pub fn last(&self) -> VLineKey {
        self.last
    }

    #[inline(always)]
    pub fn contains_key(&self, key: VLineKey) -> bool {
        self.arena.contains_key(key)
    }

    pub fn update_rope(&mut self, mut key: VLineKey, new_buffer_key: BufferKey) -> VLineKey {
        let mut line = &mut self.arena[key];
        let old_buffer_key = line.buffer_key;
        let new_start_byte = line.start_byte;
        loop {
            line.buffer_key = new_buffer_key;
            line.start_byte -= new_start_byte;
            line.end_byte -= new_start_byte;
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
}

impl VLine {
    #[inline(always)]
    pub fn slice<'r>(&self, ropes: &'r RopeMap) -> RopeSlice<'r> {
        ropes[self.buffer_key].byte_slice(self.start_byte..self.end_byte)
    }
}

pub struct SliceIterator<'a, 'r> {
    arena: &'a SlotMap<VLineKey, VLine>,
    ropes: &'r RopeMap,
    index: VLineKey,
    end: VLineKey,
}

impl<'a, 'r: 'a> Iterator for SliceIterator<'a, 'r> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.end {
            return None;
        }
        let line = self.arena.get(self.index)?;
        self.index = line.next;
        Some(line.slice(&self.ropes))
    }
}
