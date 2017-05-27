use ante::type_inference::types::{PrimitiveType, Type};
use ropey::Rope;
use tower_lsp::lsp_types::*;

pub fn position_to_index(position: Position, rope: &Rope) -> Result<usize, ropey::Error> {
    let line_start = rope.try_line_to_char(position.line as usize)?;
    let base_u16 = rope.char_to_utf16_cu(line_start);
    // utf16_cu_to_char panics out of bounds; a character past EOL falls back to rope end.
    let target_u16 = (base_u16 + position.character as usize).min(rope.len_utf16_cu());
    Ok(rope.utf16_cu_to_char(target_u16))
}

pub fn index_to_position(index: usize, rope: &Rope) -> Result<Position, ropey::Error> {
    let line = rope.try_char_to_line(index)?;
    let line_start = rope.line_to_char(line);
    let char = rope.char_to_utf16_cu(index) - rope.char_to_utf16_cu(line_start);
    Ok(Position { line: line as u32, character: char as u32 })
}

pub fn lsp_range_to_rope_range(range: Range, rope: &Rope) -> Result<std::ops::Range<usize>, ropey::Error> {
    let start = position_to_index(range.start, rope)?;
    let end = position_to_index(range.end, rope)?;
    Ok(start..end)
}

pub fn rope_range_to_lsp_range(range: std::ops::Range<usize>, rope: &Rope) -> Result<Range, ropey::Error> {
    let start = index_to_position(range.start, rope)?;
    let end = index_to_position(range.end, rope)?;
    Ok(Range { start, end })
}

/// Convert an LSP `Position` (line + UTF-16 code-unit offset) to a byte offset
/// in the underlying file, as used by the Ante compiler's `Position::byte_index`.
pub fn position_to_byte_offset(position: Position, rope: &Rope) -> Option<usize> {
    let char_idx = position_to_index(position, rope).ok()?;
    Some(rope.char_to_byte(char_idx))
}

/// Convert a byte-indexed span (as produced by the Ante compiler's `Position::byte_index`)
/// to an LSP `Range`. Clamps indices to the rope length to avoid panics.
pub fn byte_range_to_lsp_range(start_byte: usize, end_byte: usize, rope: &Rope) -> Result<Range, ropey::Error> {
    let len = rope.len_bytes();
    let start_char = rope.byte_to_char(start_byte.min(len));
    let end_char = rope.byte_to_char(end_byte.min(len));
    rope_range_to_lsp_range(start_char..end_char, rope)
}

/// Tracks the tightest span seen so far that contains `byte_offset`
pub struct SpanSearcher {
    byte_offset: usize,
    best_span_len: usize,
}

impl SpanSearcher {
    pub fn new(byte_offset: usize) -> Self {
        Self { byte_offset, best_span_len: usize::MAX }
    }

    /// Returns `true` if `[start, end)` contains `byte_offset` and is strictly
    /// tighter than every previous accepted span. The caller is expected to record
    /// the matched candidate when this returns `true`.
    pub fn try_offer(&mut self, start: usize, end: usize) -> bool {
        if start <= self.byte_offset && self.byte_offset < end {
            let span_len = end - start;
            if span_len < self.best_span_len {
                self.best_span_len = span_len;
                return true;
            }
        }
        false
    }
}

/// Types to avoid showing to users
pub fn is_internal_only_type(typ: &Type) -> bool {
    matches!(typ, Type::Primitive(PrimitiveType::Error | PrimitiveType::NoClosureEnv))
}

/// Walk backward from `byte_offset` through any identifier characters and return that prefix.
/// Empty if the cursor isn't adjacent to an identifier.
pub fn identifier_prefix_before(rope: &Rope, byte_offset: usize) -> String {
    let byte_offset = byte_offset.min(rope.len_bytes());
    let char_idx = rope.byte_to_char(byte_offset);
    let mut chars = rope.chars_at(char_idx);
    let mut collected: Vec<char> = Vec::new();
    while let Some(c) = chars.prev() {
        if c.is_alphanumeric() || c == '_' {
            collected.push(c);
        } else {
            break;
        }
    }
    collected.iter().rev().collect()
}

/// Join doc-comment lines into a single [Documentation] separated by newlines.
pub fn format_doc_comments(comments: &[String]) -> Option<Documentation> {
    if comments.is_empty() {
        return None;
    }
    Some(Documentation::MarkupContent(MarkupContent { kind: MarkupKind::Markdown, value: comments.join("\n") }))
}
