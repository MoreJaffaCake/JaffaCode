use super::view::*;
use ropey::*;
use slotmap::*;

new_key_type! {
    pub struct Key;
}

#[derive(derive_more::Debug, Default)]
pub struct VLines {
    #[debug(skip)]
    arena: SlotMap<Key, Line>,
    #[debug(skip)]
    empty: Key,
    #[debug(skip)]
    first: Key,
    wrap_at: usize,
}

impl VLines {
    pub fn new(wrap_at: usize, rope: &Rope) -> Self {
        let mut arena = SlotMap::<Key, Line>::with_key();
        let empty = arena.insert_with_key(|k| Line {
            prev: k,
            next: k,
            start_byte: 0,
            end: 0,
        });
        arena.remove(empty);

        let mut instance = Self {
            arena,
            empty,
            first: empty,
            wrap_at,
        };
        let mut it = rope.lines();

        let (mut prev, mut prev_end) = {
            let line = it.next().unwrap();
            let end = line.len_bytes();
            let key = instance.arena.insert(Line {
                prev: empty,
                next: empty,
                start_byte: 0,
                end,
            });
            let key = instance.wrap(key, rope);
            (key, end)
        };

        instance.first = prev;

        for line in it {
            let len_bytes = line.len_bytes();
            if len_bytes == 0 {
                break;
            }
            let end = prev_end + len_bytes;
            let key = instance.arena.insert(Line {
                prev: prev,
                next: empty,
                start_byte: prev_end,
                end,
            });
            instance.arena[prev].next = key;
            let key = instance.wrap(key, rope);
            prev_end = end;
            prev = key;
        }

        instance
    }

    pub fn wrap_at(&self) -> usize {
        self.wrap_at
    }

    fn wrap(&mut self, mut key: Key, rope: &Rope) -> Key {
        loop {
            let line = &self.arena[key];
            let slice = line.slice(rope);
            let len_chars = slice.len_chars();
            if len_chars <= self.wrap_at + 1 {
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
                key = self.split_line(key, self.wrap_at);
            }
        }
        key
    }

    #[inline(always)]
    pub fn slices<'v, 'r>(&self, view: &'v View, rope: &'r Rope) -> SliceIterator<'_, 'v, 'r> {
        SliceIterator {
            arena: &self.arena,
            rope,
            index: view.start,
            view,
        }
    }

    pub fn insert(&mut self, view: &View, bytes: usize, rope: &Rope) {
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
        self.wrap(view.cursor, rope);
    }

    pub fn remove(&mut self, view: &View, bytes: usize, rope: &Rope) {
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
        self.wrap(view.cursor, rope);
    }

    pub fn insert_newlines(&mut self, view: &mut View, n: usize) {
        let mut prev = view.cursor;
        let line = &self.arena[prev];
        let mut prev_end = line.end;
        let next = line.next;
        for _ in 0..n {
            let key = self.arena.insert(Line {
                prev: prev,
                next: self.empty,
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
    fn merge_next(&mut self, key: Key) {
        let b = self.arena.remove(self.arena[key].next).unwrap();
        if let Some(c) = self.arena.get_mut(b.next) {
            c.prev = key;
        }
        let a = &mut self.arena[key];
        a.next = b.next;
        a.end = b.end;
    }

    #[inline]
    fn split_line(&mut self, key: Key, char_idx: usize) -> Key {
        let line = &self.arena[key];
        let split_byte = line.start_byte + char_idx;
        let next = line.next;
        let new_line = Line {
            prev: key,
            next,
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
    pub fn first(&self) -> Key {
        self.first
    }

    #[inline(always)]
    pub fn contains_key(&self, key: Key) -> bool {
        self.arena.contains_key(key)
    }
}

impl std::ops::Index<Key> for VLines {
    type Output = Line;

    fn index(&self, key: Key) -> &Self::Output {
        &self.arena[key]
    }
}

#[derive(derive_more::Debug)]
pub struct Line {
    #[debug("<Key>")]
    pub prev: Key,
    #[debug("<Key>")]
    pub next: Key,
    pub start_byte: usize,
    pub end: usize,
}

impl Line {
    #[inline(always)]
    pub fn slice<'r>(&self, rope: &'r Rope) -> RopeSlice<'r> {
        rope.byte_slice(self.start_byte..self.end)
    }
}

pub struct SliceIterator<'a, 'v, 'r> {
    arena: &'a SlotMap<Key, Line>,
    rope: &'r Rope,
    index: Key,
    view: &'v View,
}

impl<'a, 'v, 'r: 'a> Iterator for SliceIterator<'a, 'v, 'r> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.arena.get(self.index)?;
        self.index = line.next;
        let slice = line.slice(&self.rope);
        Some(if self.view.hscroll > 0 {
            if slice.len_chars() <= self.view.hscroll {
                slice.slice(..0)
            } else {
                slice.slice(self.view.hscroll..)
            }
        } else {
            slice
        })
    }
}
