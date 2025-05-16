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
}

impl VLines {
    pub fn new(rope: &Rope, rope_key: RopeKey, wrap_at: usize) -> Self {
        let mut arena = SlotMap::<VLineKey, VLine>::with_key();
        let empty = arena.insert_with_key(|k| VLine {
            prev: k,
            next: k,
            rope_key,
            start_byte: 0,
            end: 0,
        });
        arena.remove(empty);

        let mut instance = Self {
            arena,
            empty,
            first: empty,
        };
        let mut it = rope.lines();

        let (mut prev, mut prev_end) = {
            let line = it.next().unwrap();
            let end = line.len_bytes();
            let key = instance.arena.insert(VLine {
                prev: empty,
                next: empty,
                rope_key,
                start_byte: 0,
                end,
            });
            let key = instance.wrap(rope, key, wrap_at);
            (key, end)
        };

        instance.first = prev;

        for line in it {
            let len_bytes = line.len_bytes();
            if len_bytes == 0 {
                break;
            }
            let end = prev_end + len_bytes;
            let key = instance.arena.insert(VLine {
                prev: prev,
                next: empty,
                rope_key,
                start_byte: prev_end,
                end,
            });
            instance.arena[prev].next = key;
            let key = instance.wrap(rope, key, wrap_at);
            prev_end = end;
            prev = key;
        }

        instance
    }

    fn wrap(&mut self, rope: &Rope, mut key: VLineKey, wrap_at: usize) -> VLineKey {
        loop {
            let line = &self.arena[key];
            let slice = line.slice(rope);
            let len_chars = slice.len_chars();
            if len_chars <= wrap_at + 1 {
                let newline_idx = slice
                    .chars()
                    .enumerate()
                    .find_map(|(i, c)| (c == '\n').then_some(i));
                if newline_idx == Some(len_chars.saturating_sub(1)) {
                    break;
                } else if let Some(newline_idx) = newline_idx {
                    key = self.split_line(key, newline_idx + 1);
                } else if self.arena.contains_key(line.next) {
                    self.merge_next(key);
                } else {
                    unreachable!("missing newline at EOF");
                }
            } else {
                key = self.split_line(key, wrap_at);
            }
        }
        key
    }

    #[inline(always)]
    pub fn slices<'r>(&self, view: &View, ropes: &'r RopeMap) -> SliceIterator<'_, 'r> {
        SliceIterator {
            arena: &self.arena,
            ropes,
            index: view.start,
        }
    }

    pub fn insert(&mut self, view: &View, bytes: usize, rope: &Rope, wrap_at: usize) {
        let mut key = view.cursor;
        {
            let line = &mut self.arena[key];
            line.end += bytes;
            key = line.next;
        }
        while let Some(line) = self.arena.get_mut(key) {
            line.start_byte += bytes;
            line.end += bytes;
            key = line.next;
        }
        self.wrap(rope, view.cursor, wrap_at);
    }

    pub fn remove(&mut self, view: &View, bytes: usize, rope: &Rope, wrap_at: usize) {
        let mut key = view.cursor;
        {
            let line = &mut self.arena[key];
            line.end -= bytes;
            key = line.next;
        }
        while let Some(line) = self.arena.get_mut(key) {
            line.start_byte -= bytes;
            line.end -= bytes;
            key = line.next;
        }
        self.wrap(rope, view.cursor, wrap_at);
    }

    pub fn insert_newlines(&mut self, view: &mut View, n: usize) {
        let mut prev = view.cursor;
        let line = &self.arena[prev];
        let rope_key = line.rope_key;
        let mut prev_end = line.end;
        let next = line.next;
        for _ in 0..n {
            let key = self.arena.insert(VLine {
                prev: prev,
                next: self.empty,
                rope_key,
                start_byte: prev_end,
                end: prev_end + 1,
            });
            self.arena[prev].next = key;
            prev = key;
            prev_end += 1;
        }
        self.arena[prev].next = next;
        view.cursor = prev;
        view.cursor_idx += n;
    }

    #[inline]
    fn merge_next(&mut self, key: VLineKey) {
        let b = self.arena.remove(self.arena[key].next).unwrap();
        if let Some(c) = self.arena.get_mut(b.next) {
            c.prev = key;
        }
        let a = &mut self.arena[key];
        a.next = b.next;
        a.end = b.end;
    }

    #[inline]
    fn split_line(&mut self, key: VLineKey, char_idx: usize) -> VLineKey {
        let line = &self.arena[key];
        let split_byte = line.start_byte + char_idx;
        let next = line.next;
        let new_line = VLine {
            prev: key,
            next,
            rope_key: line.rope_key,
            start_byte: split_byte,
            end: line.end,
        };
        debug_assert!(new_line.start_byte != new_line.end);
        let new_key = self.arena.insert(new_line);
        if let Some(line) = self.arena.get_mut(next) {
            line.prev = new_key;
        }
        let line = &mut self.arena[key];
        line.end = split_byte;
        line.next = new_key;
        new_key
    }

    #[inline(always)]
    pub fn first(&self) -> VLineKey {
        self.first
    }

    #[inline(always)]
    pub fn contains_key(&self, key: VLineKey) -> bool {
        self.arena.contains_key(key)
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
    pub rope_key: RopeKey,
    pub start_byte: usize,
    pub end: usize,
}

impl VLine {
    #[inline(always)]
    pub fn slice<'r>(&self, rope: &'r Rope) -> RopeSlice<'r> {
        rope.byte_slice(self.start_byte..self.end)
    }
}

pub struct SliceIterator<'a, 'r> {
    arena: &'a SlotMap<VLineKey, VLine>,
    ropes: &'r RopeMap,
    index: VLineKey,
}

impl<'a, 'r: 'a> Iterator for SliceIterator<'a, 'r> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.arena.get(self.index)?;
        let rope = &self.ropes[line.rope_key];
        self.index = line.next;
        Some(line.slice(rope))
    }
}
