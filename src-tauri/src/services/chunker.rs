// Разбиение длинных текстов на чанки с перекрытием

pub struct Chunk {
    pub text: String,
    pub index: usize,
}

/// Result of document chunking with metadata.
pub struct ChunkResult {
    pub content: String,
    pub index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub metadata: serde_json::Value,
}

/// Разбивает текст на чанки по ~max_words слов с перекрытием overlap_words.
pub fn chunk_text(
    text: &str,
    max_words: usize,
    overlap_words: usize,
) -> Vec<Chunk> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }
    if words.len() <= max_words {
        return vec![Chunk {
            text: words.join(" "),
            index: 0,
        }];
    }
    let step = max_words.saturating_sub(overlap_words).max(1);
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while start < words.len() {
        let end = (start + max_words).min(words.len());
        let slice = &words[start..end];
        chunks.push(Chunk {
            text: slice.join(" "),
            index,
        });
        index += 1;
        if end >= words.len() {
            break;
        }
        start += step;
    }
    chunks
}

/// Main entry point for KB document chunking. Dispatches by strategy.
/// chunk_size is in tokens (~4 chars/token approximation).
pub fn chunk_document(
    text: &str,
    strategy: &str,
    chunk_size: usize,
    overlap: usize,
) -> Vec<ChunkResult> {
    if text.trim().is_empty() {
        return vec![];
    }
    match strategy {
        "headings" => chunk_by_headings(text, chunk_size),
        "paragraphs" => chunk_by_paragraphs(text, chunk_size),
        _ => chunk_by_tokens(text, chunk_size, overlap), // "tokens" or default
    }
}

/// Chunk by approximate token count using word-based splitting.
/// ~4 chars per token for English, so chunk_size tokens ≈ chunk_size * 4 / avg_word_len words.
/// We approximate: 1 token ≈ 0.75 words (conservative).
fn chunk_by_tokens(text: &str, chunk_size: usize, overlap: usize) -> Vec<ChunkResult> {
    let max_words = (chunk_size * 3 / 4).max(1); // tokens to words approximation
    let overlap_words = (overlap * 3 / 4).max(0);
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }
    if words.len() <= max_words {
        return vec![ChunkResult {
            content: text.to_string(),
            index: 0,
            start_offset: 0,
            end_offset: text.len(),
            metadata: serde_json::json!({"strategy": "tokens"}),
        }];
    }

    let step = max_words.saturating_sub(overlap_words).max(1);
    let mut results = Vec::new();
    let mut start = 0;
    let mut index = 0;

    // Build word offset map for byte offsets
    let word_offsets: Vec<(usize, usize)> = find_word_offsets(text);

    while start < words.len() {
        let end = (start + max_words).min(words.len());
        let content = words[start..end].join(" ");
        let start_offset = word_offsets.get(start).map(|w| w.0).unwrap_or(0);
        let end_offset = word_offsets.get(end - 1).map(|w| w.1).unwrap_or(text.len());

        results.push(ChunkResult {
            content,
            index,
            start_offset,
            end_offset,
            metadata: serde_json::json!({"strategy": "tokens"}),
        });
        index += 1;
        if end >= words.len() {
            break;
        }
        start += step;
    }
    results
}

/// Chunk by markdown headings (# ## ### etc.)
fn chunk_by_headings(text: &str, max_chunk_size: usize) -> Vec<ChunkResult> {
    let max_words = (max_chunk_size * 3 / 4).max(1);
    let heading_re = regex::Regex::new(r"(?m)^(#{1,6})\s+(.+)$").unwrap();

    let mut sections: Vec<(String, usize, usize)> = Vec::new(); // (heading, start, end)
    let mut last_start = 0;
    let mut last_heading = String::new();

    for m in heading_re.find_iter(text) {
        if m.start() > last_start || !last_heading.is_empty() {
            sections.push((last_heading.clone(), last_start, m.start()));
        }
        last_heading = m.as_str().to_string();
        last_start = m.start();
    }
    // Last section
    if last_start < text.len() {
        sections.push((last_heading, last_start, text.len()));
    }

    if sections.is_empty() {
        // No headings found, fall back to paragraphs
        return chunk_by_paragraphs(text, max_chunk_size);
    }

    let mut results = Vec::new();
    let mut index = 0;
    for (heading, start, end) in &sections {
        let section_text = text[*start..*end].trim();
        if section_text.is_empty() {
            continue;
        }
        let words: Vec<&str> = section_text.split_whitespace().collect();
        if words.len() <= max_words {
            results.push(ChunkResult {
                content: section_text.to_string(),
                index,
                start_offset: *start,
                end_offset: *end,
                metadata: serde_json::json!({"strategy": "headings", "heading": heading}),
            });
            index += 1;
        } else {
            // Sub-split large sections by paragraphs
            let sub_chunks = chunk_by_paragraphs(section_text, max_chunk_size);
            for mut sc in sub_chunks {
                sc.index = index;
                sc.start_offset += start;
                sc.end_offset = sc.end_offset.min(*end - *start) + *start;
                sc.metadata = serde_json::json!({"strategy": "headings", "heading": heading});
                results.push(sc);
                index += 1;
            }
        }
    }
    results
}

/// Chunk by paragraphs (double newline separated), merging short ones.
fn chunk_by_paragraphs(text: &str, max_chunk_size: usize) -> Vec<ChunkResult> {
    let max_words = (max_chunk_size * 3 / 4).max(1);
    let paragraphs: Vec<&str> = text.split("\n\n").collect();

    let mut results = Vec::new();
    let mut current_content = String::new();
    let mut current_words = 0usize;
    let mut current_start = 0usize;
    let mut offset = 0usize;
    let mut index = 0;
    let mut para_num = 0;

    for para in &paragraphs {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            offset += para.len() + 2; // +2 for \n\n
            continue;
        }
        let para_words = trimmed.split_whitespace().count();

        if current_words > 0 && current_words + para_words > max_words {
            // Flush current chunk
            results.push(ChunkResult {
                content: current_content.trim().to_string(),
                index,
                start_offset: current_start,
                end_offset: offset,
                metadata: serde_json::json!({"strategy": "paragraphs", "paragraph": para_num}),
            });
            index += 1;
            current_content = String::new();
            current_words = 0;
            current_start = offset;
        }

        if current_content.is_empty() {
            current_start = offset;
        }
        if !current_content.is_empty() {
            current_content.push_str("\n\n");
        }
        current_content.push_str(trimmed);
        current_words += para_words;
        para_num += 1;

        offset += para.len() + 2; // +2 for \n\n separator
    }

    // Flush remaining
    if !current_content.is_empty() {
        results.push(ChunkResult {
            content: current_content.trim().to_string(),
            index,
            start_offset: current_start,
            end_offset: text.len(),
            metadata: serde_json::json!({"strategy": "paragraphs", "paragraph": para_num}),
        });
    }

    if results.is_empty() && !text.trim().is_empty() {
        results.push(ChunkResult {
            content: text.trim().to_string(),
            index: 0,
            start_offset: 0,
            end_offset: text.len(),
            metadata: serde_json::json!({"strategy": "paragraphs", "paragraph": 0}),
        });
    }

    results
}

/// Find byte offsets of each word in text (start, end).
fn find_word_offsets(text: &str) -> Vec<(usize, usize)> {
    let mut offsets = Vec::new();
    let mut in_word = false;
    let mut word_start = 0;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if in_word {
                offsets.push((word_start, i));
                in_word = false;
            }
        } else if !in_word {
            word_start = i;
            in_word = true;
        }
    }
    if in_word {
        offsets.push((word_start, text.len()));
    }
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_empty() {
        assert!(chunk_text("", 300, 50).is_empty());
    }

    #[test]
    fn chunk_short() {
        let c = chunk_text("one two three", 300, 50);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].text, "one two three");
        assert_eq!(c[0].index, 0);
    }

    #[test]
    fn chunk_with_overlap() {
        let words: Vec<String> = (0..10).map(|i| format!("w{}", i)).collect();
        let text = words.join(" ");
        let c = chunk_text(&text, 4, 2);
        assert!(c.len() >= 2);
        assert_eq!(c[0].index, 0);
        assert_eq!(c[1].index, 1);
    }

    #[test]
    fn chunk_document_tokens() {
        let text = "word ".repeat(100);
        let chunks = chunk_document(&text, "tokens", 50, 10);
        assert!(!chunks.is_empty());
        for c in &chunks {
            assert!(!c.content.is_empty());
        }
    }

    #[test]
    fn chunk_document_paragraphs() {
        let text = "Paragraph one.\n\nParagraph two.\n\nParagraph three.";
        let chunks = chunk_document(&text, "paragraphs", 1000, 0);
        assert_eq!(chunks.len(), 1); // All fit in one chunk
    }

    #[test]
    fn chunk_document_headings() {
        let text = "# Title\n\nIntro text.\n\n## Section 1\n\nContent 1.\n\n## Section 2\n\nContent 2.";
        let chunks = chunk_document(&text, "headings", 1000, 0);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn chunk_russian_text_utf8_safe() {
        let text = "Привет мир. Это тест русского текста. Юникод работает корректно. Ещё одно предложение для проверки.";
        let chunks = chunk_text(text, 3, 1);
        assert!(!chunks.is_empty());
        for c in &chunks {
            // Every chunk must be valid UTF-8 (String guarantees this, but verify content)
            assert!(!c.text.is_empty());
            assert!(c.text.is_char_boundary(c.text.len()));
        }
    }

    #[test]
    fn chunk_very_long_line_no_separators() {
        let text = "a".repeat(10000);
        // No whitespace → single "word" of 10000 chars
        let chunks = chunk_text(&text, 100, 10);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, text);
    }

    #[test]
    fn chunk_only_whitespace() {
        let text = "   \n\n  \t  ";
        let chunks = chunk_text(text, 100, 10);
        assert!(chunks.is_empty());
    }

    #[test]
    fn chunk_size_one() {
        let text = "one two three four five";
        let chunks = chunk_text(text, 1, 0);
        assert_eq!(chunks.len(), 5);
        assert_eq!(chunks[0].text, "one");
        assert_eq!(chunks[4].text, "five");
    }

    #[test]
    fn find_word_offsets_russian() {
        let text = "Привет мир";
        let offsets = find_word_offsets(text);
        assert_eq!(offsets.len(), 2);
        // "Привет" is 12 bytes in UTF-8 (6 chars × 2 bytes each)
        assert_eq!(offsets[0].0, 0);
        assert_eq!(offsets[0].1, 12); // "Привет" ends at byte 12
        assert_eq!(&text[offsets[0].0..offsets[0].1], "Привет");
        assert_eq!(&text[offsets[1].0..offsets[1].1], "мир");
    }

    #[test]
    fn find_word_offsets_empty() {
        let offsets = find_word_offsets("");
        assert!(offsets.is_empty());
    }

    #[test]
    fn chunk_document_unknown_strategy_falls_back_to_tokens() {
        let text = "word ".repeat(50);
        let chunks = chunk_document(&text, "unknown_strategy", 20, 5);
        assert!(!chunks.is_empty());
        // Should use tokens strategy (default)
        let meta = &chunks[0].metadata;
        assert_eq!(meta["strategy"], "tokens");
    }

    #[test]
    fn chunk_headings_no_headings_falls_back() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = chunk_document(text, "headings", 1000, 0);
        // No headings → falls back to paragraph-based chunking
        assert!(!chunks.is_empty());
        // Content should still be chunked (may use paragraphs internally)
        let all_content: String = chunks.iter().map(|c| c.content.clone()).collect();
        assert!(all_content.contains("First paragraph"));
        assert!(all_content.contains("Third paragraph"));
    }
}
