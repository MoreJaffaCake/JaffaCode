use super::*;

#[derive(derive_more::Debug)]
pub struct Buffer {
    #[debug(skip)]
    pub key: BufferKey,
    #[debug(skip)]
    pub start: VLineKey,
    #[debug(skip)]
    pub end: VLineKey,
    pub wrap_at: usize,
    pub indent: usize,
}

impl Buffer {
    pub fn new(
        key: BufferKey,
        start: VLineKey,
        end: VLineKey,
        wrap_at: usize,
        indent: usize,
    ) -> Self {
        debug_assert!(wrap_at < HSPACES.len());
        Self {
            key,
            start,
            end,
            wrap_at,
            indent,
        }
    }

    #[inline]
    pub fn insert(
        &self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        char_idx: usize,
        text: &str,
        vline_key: VLineKey,
    ) -> VLineKey {
        ropes[self.key].insert(char_idx, text);
        vlines.insert(ropes, vline_key, text.len(), self.wrap_at)
    }

    #[inline]
    pub fn insert_char(
        &self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        char_idx: usize,
        c: char,
        vline_key: VLineKey,
    ) {
        let rope = &mut ropes[self.key];
        let len_chars_before = rope.len_chars();
        rope.insert_char(char_idx, c);
        let bytes = rope.len_chars() - len_chars_before;
        vlines.insert(ropes, vline_key, bytes, self.wrap_at);
    }

    #[inline]
    pub fn remove(
        &self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        char_idx: usize,
        vline_key: VLineKey,
    ) {
        let rope = &mut ropes[self.key];
        let len_chars_before = rope.len_chars();
        rope.remove(char_idx..=char_idx);
        let bytes = len_chars_before - rope.len_chars();
        vlines.remove(ropes, vline_key, bytes, self.wrap_at);
    }

    pub fn rewrap(&self, vlines: &mut VLines, ropes: &RopeMap) {
        let mut key = self.start;
        loop {
            key = vlines.wrap(ropes, key, self.wrap_at);
            let next = vlines[key].next;
            if next == self.end {
                break;
            }
            key = next;
        }
    }
}
