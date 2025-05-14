use ropey::*;
use slotmap::*;

new_key_type! {
    struct Index;
}

#[derive(Debug, Default)]
pub struct VisualLines {
    arena: SlotMap<Index, Line>,
    start: Option<Index>,
    cursor: Option<Index>,
    cursor_idx: usize,
}

impl VisualLines {
    pub fn regenerate(&mut self, rope: &Rope, wrap_at: usize) {
        let mut it = rope.lines();

        let (mut prev, mut prev_end) = {
            let Some(line) = it.next() else {
                return;
            };
            let end = line.len_bytes();
            let key = self.arena.insert(Line {
                prev: None,
                next: None,
                start: 0,
                end,
            });
            let key = self.wrap(key, rope, wrap_at);
            (key, end)
        };

        self.start = Some(prev);
        self.cursor = Some(prev);
        self.cursor_idx = 0;

        for line in it {
            let len_bytes = line.len_bytes();
            if len_bytes == 0 {
                break;
            }
            let end = prev_end + len_bytes;
            let key = self.arena.insert(Line {
                prev: Some(prev),
                next: None,
                start: prev_end,
                end,
            });
            self.arena[prev].next = Some(key);
            let key = self.wrap(key, rope, wrap_at);
            prev_end = end;
            prev = key;
        }
    }

    fn wrap(&mut self, mut key: Index, rope: &Rope, wrap_at: usize) -> Index {
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
                } else if line.next.is_some() {
                    self.merge_next(key);
                } else {
                    todo!("no EOF new line")
                }
            } else {
                key = self.split_line(key, wrap_at);
            }
        }
        key
    }

    #[inline(always)]
    pub fn iter(&self) -> LineIterator {
        LineIterator {
            arena: &self.arena,
            index: self.start,
        }
    }

    #[inline(always)]
    pub fn cursor(&self) -> &Line {
        &self.arena[self.cursor.unwrap()]
    }

    #[inline(always)]
    pub fn cursor_idx(&self) -> usize {
        self.cursor_idx
    }

    #[inline(always)]
    pub fn move_cursor_prev(&mut self) {
        self.cursor = self.arena[self.cursor.unwrap()].prev;
        self.cursor_idx -= 1;
    }

    #[inline(always)]
    pub fn move_cursor_next(&mut self) {
        if let Some(next) = self.arena[self.cursor.unwrap()].next {
            self.cursor = Some(next);
            self.cursor_idx += 1;
        }
    }

    pub fn insert(&mut self, bytes: usize, rope: &Rope, wrap_at: usize) {
        let mut key = self.cursor;
        let cursor = key.unwrap();
        {
            let line = &mut self.arena[key.unwrap()];
            line.end += bytes;
            key = line.next;
        }
        while key.is_some() {
            let line = &mut self.arena[key.unwrap()];
            line.start += bytes;
            line.end += bytes;
            key = line.next;
        }
        self.wrap(cursor, rope, wrap_at);
    }

    pub fn remove(&mut self, bytes: usize, rope: &Rope, wrap_at: usize) {
        let mut key = self.cursor;
        let cursor = key.unwrap();
        {
            let line = &mut self.arena[key.unwrap()];
            line.end -= bytes;
            key = line.next;
        }
        while key.is_some() {
            let line = &mut self.arena[key.unwrap()];
            line.start -= bytes;
            line.end -= bytes;
            key = line.next;
        }
        self.wrap(cursor, rope, wrap_at);
    }

    pub fn insert_newlines(&mut self, n: usize) {
        let mut prev = self.cursor.unwrap();
        let line = &self.arena[prev];
        let mut prev_end = line.end;
        let next = line.next;
        for _ in 0..n {
            let key = self.arena.insert(Line {
                prev: Some(prev),
                next: None,
                start: prev_end,
                end: prev_end + 1,
            });
            self.arena[prev].next = Some(key);
            prev = key;
            prev_end += 1;
        }
        self.arena[prev].next = next;
        self.cursor = Some(prev);
        self.cursor_idx += n;
    }

    #[inline]
    fn merge_next(&mut self, key: Index) {
        let b = self.arena.remove(self.arena[key].next.unwrap()).unwrap();
        if let Some(c_key) = b.next {
            self.arena[c_key].prev = Some(key);
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
            prev: Some(key),
            next,
            start: split_byte,
            end: line.end,
        };
        debug_assert!(new_line.start != new_line.end);
        let new_key = self.arena.insert(new_line);
        if let Some(next) = next {
            self.arena[next].prev = Some(new_key);
        }
        let line = &mut self.arena[key];
        line.end = split_byte;
        line.next = Some(new_key);
        new_key
    }
}

#[derive(Debug)]
pub struct Line {
    prev: Option<Index>,
    next: Option<Index>,
    pub start: usize,
    pub end: usize,
}

impl Line {
    #[inline(always)]
    pub fn slice<'r>(&self, rope: &'r Rope) -> RopeSlice<'r> {
        rope.byte_slice(self.start..self.end)
    }
}

pub struct LineIterator<'a> {
    arena: &'a SlotMap<Index, Line>,
    index: Option<Index>,
}

impl<'a> Iterator for LineIterator<'a> {
    type Item = &'a Line;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.arena.get(self.index?)?;
        self.index = line.next;
        Some(line)
    }
}
