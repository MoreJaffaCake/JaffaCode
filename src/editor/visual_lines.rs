use generational_arena::*;
use ropey::*;

#[derive(Debug, Default)]
pub struct VisualLines {
    arena: Arena<Line>,
    start: Option<Index>,
    cursor: Option<Index>,
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
            debug_assert!(len_chars > 0);
            if len_chars <= wrap_at + 1 {
                let newline_idx = slice
                    .chars()
                    .enumerate()
                    .find_map(|(i, c)| (c == '\n').then_some(i));
                if newline_idx == Some(len_chars - 1) {
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

    pub fn iter(&self) -> LineIterator {
        LineIterator {
            arena: &self.arena,
            index: self.start,
        }
    }

    pub fn cursor(&self) -> &Line {
        &self.arena[self.cursor.unwrap()]
    }

    pub fn move_cursor_prev(&mut self) {
        self.cursor = self.arena[self.cursor.unwrap()].prev
    }

    pub fn move_cursor_next(&mut self) {
        self.cursor = self.arena[self.cursor.unwrap()].next
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
    arena: &'a Arena<Line>,
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
