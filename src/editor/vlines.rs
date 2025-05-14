use super::view::*;
use ropey::*;
use slotmap::*;

new_key_type! {
    pub struct Index;
}

#[derive(derive_more::Debug, Default)]
pub struct VLines {
    #[debug(skip)]
    arena: SlotMap<Index, Line>,
    #[debug(skip)]
    empty: Index,
    #[debug(skip)]
    start: Index,
    wrap_at: usize,
}

impl VLines {
    pub fn new(wrap_at: usize, rope: &Rope) -> Self {
        let mut arena = SlotMap::<Index, Line>::with_key();
        let empty = arena.insert_with_key(|k| Line {
            prev: k,
            next: k,
            start: 0,
            end: 0,
        });
        arena.remove(empty);

        let mut instance = Self {
            arena,
            empty,
            start: empty,
            wrap_at,
        };
        let mut it = rope.lines();

        let (mut prev, mut prev_end) = {
            let line = it.next().unwrap();
            let end = line.len_bytes();
            let key = instance.arena.insert(Line {
                prev: empty,
                next: empty,
                start: 0,
                end,
            });
            let key = instance.wrap(key, rope);
            (key, end)
        };

        instance.start = prev;

        for line in it {
            let len_bytes = line.len_bytes();
            if len_bytes == 0 {
                break;
            }
            let end = prev_end + len_bytes;
            let key = instance.arena.insert(Line {
                prev: prev,
                next: empty,
                start: prev_end,
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

    fn wrap(&mut self, mut key: Index, rope: &Rope) -> Index {
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
                    unreachable!();
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
            line.start += bytes;
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
            line.start -= bytes;
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
                start: prev_end,
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
    fn merge_next(&mut self, key: Index) {
        let b = self.arena.remove(self.arena[key].next).unwrap();
        if let Some(c) = self.arena.get_mut(b.next) {
            c.prev = key;
        }
        let a = &mut self.arena[key];
        a.next = b.next;
        a.end = b.end;
    }

    #[inline]
    fn split_line(&mut self, key: Index, char_idx: usize) -> Index {
        let line = &self.arena[key];
        let split_byte = line.start + char_idx;
        let next = line.next;
        let new_line = Line {
            prev: key,
            next,
            start: split_byte,
            end: line.end,
        };
        debug_assert!(new_line.start != new_line.end);
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
    pub fn start(&self) -> Index {
        self.start
    }

    #[inline(always)]
    pub fn contains_key(&self, key: Index) -> bool {
        self.arena.contains_key(key)
    }
}

impl std::ops::Index<Index> for VLines {
    type Output = Line;

    fn index(&self, key: Index) -> &Self::Output {
        &self.arena[key]
    }
}

#[derive(derive_more::Debug)]
pub struct Line {
    #[debug("<Index>")]
    pub prev: Index,
    #[debug("<Index>")]
    pub next: Index,
    pub start: usize,
    pub end: usize,
}

impl Line {
    #[inline(always)]
    pub fn slice<'r>(&self, rope: &'r Rope) -> RopeSlice<'r> {
        rope.byte_slice(self.start..self.end)
    }
}

pub struct SliceIterator<'a, 'v, 'r> {
    arena: &'a SlotMap<Index, Line>,
    rope: &'r Rope,
    index: Index,
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
