// Разбиение длинных текстов на чанки с перекрытием

pub struct Chunk {
    pub text: String,
    pub index: usize,
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
}
