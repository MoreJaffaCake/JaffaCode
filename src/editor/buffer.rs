use super::*;

#[derive(derive_more::Debug)]
pub struct Buffer {
    #[debug(skip)]
    pub buffer_key: BufferKey,
    #[debug(skip)]
    pub start: VLineKey,
    pub end: VLineKey,
    pub wrap_at: usize,
    pub indent: usize,
}

impl Buffer {
    pub fn new(
        buffer_key: BufferKey,
        start: VLineKey,
        end: VLineKey,
        wrap_at: usize,
        indent: usize,
    ) -> Self {
        debug_assert!(wrap_at < MAX_WRAP_AT);
        Self {
            buffer_key,
            start,
            end,
            wrap_at,
            indent,
        }
    }

    pub fn line_len(vlines: &VLines, ropes: &RopeMap, buffers: &BufferMap, key: VLineKey) -> usize {
        let line = &vlines[key];
        let line_len = line.slice(ropes).len_chars().saturating_sub(1);
        let buffer = &buffers[line.buffer_key];
        let indent = buffer.indent;
        line_len + indent
    }

    #[inline]
    pub fn insert(
        &self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        char_idx: usize,
        text: &str,
        vline_key: VLineKey,
    ) {
        ropes[self.buffer_key].insert(char_idx, text);
        vlines.insert(ropes, vline_key, text.len(), self.wrap_at);
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
        let rope = &mut ropes[self.buffer_key];
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
        let rope = &mut ropes[self.buffer_key];
        let len_chars_before = rope.len_chars();
        rope.remove(char_idx..=char_idx);
        let bytes = len_chars_before - rope.len_chars();
        vlines.remove(ropes, vline_key, bytes, self.wrap_at);
    }
}
