use super::*;

#[derive(derive_more::Debug)]
pub struct Buffer {
    #[debug(skip)]
    pub key: BufferKey,
    #[debug(skip)]
    pub start: VLineCursor,
    #[debug(skip)]
    pub end: VLineCursor,
    pub wrap_at: usize,
    pub indent: usize,
}

impl Buffer {
    pub fn new(
        key: BufferKey,
        start: VLineCursor,
        end: VLineCursor,
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
        cursor: VLineCursor,
    ) -> VLineKey {
        ropes[self.key].insert(char_idx, text);
        cursor.insert(vlines, ropes, text.len(), self.wrap_at)
    }

    #[inline]
    pub fn insert_char(
        &self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        char_idx: usize,
        c: char,
        cursor: VLineCursor,
    ) {
        let rope = &mut ropes[self.key];
        let len_chars_before = rope.len_chars();
        rope.insert_char(char_idx, c);
        let bytes = rope.len_chars() - len_chars_before;
        cursor.insert(vlines, ropes, bytes, self.wrap_at);
    }

    #[inline]
    pub fn remove(
        &self,
        vlines: &mut VLines,
        ropes: &mut RopeMap,
        char_idx: usize,
        cursor: VLineCursor,
    ) {
        let rope = &mut ropes[self.key];
        let len_chars_before = rope.len_chars();
        rope.remove(char_idx..=char_idx);
        let bytes = len_chars_before - rope.len_chars();
        cursor.remove(vlines, ropes, bytes, self.wrap_at);
    }

    pub fn rewrap(&self, vlines: &mut VLines, ropes: &RopeMap) {
        let mut cursor = self.start;
        loop {
            cursor = cursor.rewrap(vlines, ropes, self.wrap_at);
            if !cursor.move_next_logical_if(vlines, |cur| cur != self.end) {
                break;
            }
        }
    }

    pub fn indent(&mut self, vlines: &mut VLines, ropes: &RopeMap) {
        self.indent += INDENT;
        self.wrap_at = (WRAP_AT - self.indent).max(MIN_WRAP_AT);
        self.rewrap(vlines, ropes);
    }

    pub fn dedent(&mut self, vlines: &mut VLines, ropes: &RopeMap) {
        self.indent -= INDENT;
        self.wrap_at = WRAP_AT - self.indent;
        self.rewrap(vlines, ropes);
    }

    pub fn find_next_block(
        &self,
        vlines: &VLines,
        ropes: &RopeMap,
        buffers: &BufferMap,
    ) -> Option<(VLineCursor, usize, usize)> {
        let cursor = self.end;
        if cursor.is_null() {
            return None;
        }
        let buffer = &buffers[vlines[cursor].buffer_key];
        let relative_indent = cursor
            .iter_logical(vlines)
            .end_bounded(buffer.end)
            .find_map(|cur| cur.detect_indent(vlines, ropes))
            .unwrap_or(0);
        let total_indent = buffer.indent + relative_indent;
        Some((cursor, relative_indent, total_indent))
    }

    pub fn find_prev_block(
        &self,
        vlines: &VLines,
        ropes: &RopeMap,
        buffers: &BufferMap,
    ) -> Option<(VLineCursor, usize, usize)> {
        let cursor = self.start.peek_prev_logical(vlines)?;
        let buffer = &buffers[vlines[cursor].buffer_key];
        let relative_indent = cursor
            .iter_logical(vlines)
            .reversed()
            .start_bounded(buffer.start)
            .find_map(|cur| cur.detect_indent(vlines, ropes))
            .unwrap_or(0);
        let total_indent = buffer.indent + relative_indent;
        Some((cursor, relative_indent, total_indent))
    }
}
