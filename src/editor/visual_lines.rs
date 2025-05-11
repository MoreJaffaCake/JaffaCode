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
            (
                self.arena.insert(Line {
                    prev: None,
                    next: None,
                    start: 0,
                    end,
                }),
                end,
            )
        };

        self.start = Some(prev);
        self.cursor = Some(prev);

        for line in it {
            let end = prev_end + line.len_bytes();
            let key = self.arena.insert(Line {
                prev: Some(prev),
                next: None,
                start: prev_end,
                end,
            });
            self.arena[prev].next = Some(key);
            prev_end = end;
            prev = key;
        }
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

    pub fn insert(&mut self, bytes: usize) {
        let mut key = self.cursor;
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
