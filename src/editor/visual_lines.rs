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
                if slice.char(len_chars - 1) == '\n' {
                    break;
                } else if let Some(next) = line.next {
                    let next_line = self.arena.remove(next).unwrap();
                    let line = &mut self.arena[key];
                    line.next = next_line.next;
                    line.end = next_line.end;
                    if let Some(next_next) = next_line.next {
                        self.arena[next_next].prev = Some(key);
                    }
                } else {
                    todo!("no EOF new line")
                }
            } else {
                let new_line = Line {
                    prev: Some(key),
                    next: line.next,
                    start: line.start + slice.char_to_byte(wrap_at),
                    end: line.end,
                };
                debug_assert!(new_line.start != new_line.end);
                let new_key = self.arena.insert(new_line);
                let line = &mut self.arena[key];
                line.end = line.start + slice.char_to_byte(wrap_at);
                line.next = Some(new_key);
                key = new_key;
            }
        }
        key
    }

    pub fn start(&self) -> &Line {
        &self.arena[self.start.unwrap()]
    }

    pub fn next_line(&self, line: &Line) -> Option<&Line> {
        Some(&self.arena[line.next?])
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
        let _ = self.wrap(cursor, rope, wrap_at);
    }

    pub fn remove(&mut self, bytes: usize) {
        let mut key = self.cursor;
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
    }

    pub fn merge_next(&mut self) -> &Line {
        let a_key = self.cursor.unwrap();
        let b = self.arena.remove(self.arena[a_key].next.unwrap()).unwrap();
        if let Some(c_key) = b.next {
            self.arena[c_key].prev = Some(a_key);
        }
        let a = &mut self.arena[a_key];
        a.next = b.next;
        a.end = b.end;
        a
    }

    pub fn merge_prev(&mut self) -> &Line {
        let c_key = self.cursor.unwrap();
        let b_key = self.arena[c_key].prev.unwrap();
        let b = self.arena.remove(b_key).unwrap();
        if let Some(a_key) = b.prev {
            self.arena[a_key].next = Some(c_key);
        }
        if self.start.unwrap() == b_key {
            self.start = Some(c_key);
        }
        let c = &mut self.arena[c_key];
        c.prev = b.prev;
        c.start = b.start;
        c
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
