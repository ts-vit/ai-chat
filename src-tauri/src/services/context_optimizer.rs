// Context Optimizer: sentence extraction, redundancy removal, and token budget
// Sits between search orchestrator output and RAG context building

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::services::kb_search::KbSearchResultItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextOptimizationConfig {
    pub enabled: bool,
    pub token_budget: usize,
    pub sentence_extraction: bool,
    pub redundancy_removal: bool,
    pub redundancy_threshold: f32,
    pub min_sentences_per_chunk: usize,
    pub max_sentences_per_chunk: usize,
}

impl Default for ContextOptimizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            token_budget: 4000,
            sentence_extraction: true,
            redundancy_removal: true,
            redundancy_threshold: 0.85,
            min_sentences_per_chunk: 2,
            max_sentences_per_chunk: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimizedChunk {
    pub chunk_id: String,
    pub document_id: String,
    pub document_name: String,
    pub original_content: String,
    pub optimized_content: String,
    pub chunk_index: i64,
    pub score: f32,
    pub metadata: serde_json::Value,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationTrace {
    pub original_tokens: usize,
    pub after_sentence_extraction: usize,
    pub after_redundancy_removal: usize,
    pub final_tokens: usize,
    pub chunks_before: usize,
    pub chunks_after: usize,
}

fn estimate_tokens(text: &str) -> usize {
    (text.len() + 2) / 3
}

fn tokenize_words(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() > 2)
        .map(|w| w.to_string())
        .collect()
}

/// Split text into sentences using simple heuristics.
/// Handles English and Russian punctuation.
fn split_sentences(text: &str) -> Vec<String> {
    if text.trim().is_empty() {
        return vec![];
    }

    // First split on paragraph boundaries
    let paragraphs: Vec<&str> = text.split("\n\n").collect();

    let mut sentences = Vec::new();

    for para in &paragraphs {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Split within paragraph on sentence-ending punctuation followed by space + uppercase
        let mut current_start = 0;
        let chars: Vec<char> = trimmed.chars().collect();
        let len = chars.len();

        let mut i = 0;
        while i < len {
            if (chars[i] == '.' || chars[i] == '?' || chars[i] == '!') && i + 1 < len {
                // Check for abbreviations: skip if preceded by a single uppercase letter (e.g., "Dr.")
                // or common abbreviations
                let before_dot = if i > 0 {
                    let prefix: String = chars[..i].iter().collect();
                    let last_word = prefix.split_whitespace().next_back().unwrap_or("");
                    last_word
                        .to_string()
                        .chars()
                        .all(|c| c.is_lowercase() && c.is_alphabetic())
                        && last_word.len() <= 4
                        && matches!(
                            last_word,
                            "e.g" | "i.e" | "vs" | "dr" | "mr" | "mrs" | "ms" | "prof" | "etc"
                                | "т.д" | "т.п" | "др" | "г" | "гг"
                        )
                } else {
                    false
                };

                if before_dot && chars[i] == '.' {
                    i += 1;
                    continue;
                }

                // Check next char is whitespace or newline
                if i + 1 < len && (chars[i + 1] == ' ' || chars[i + 1] == '\n') {
                    // Check char after space is uppercase, digit, or quote
                    let next_content = if i + 2 < len {
                        chars[i + 2].is_uppercase() || chars[i + 2].is_ascii_digit()
                            || chars[i + 2] == '"' || chars[i + 2] == '\''
                            || chars[i + 2] == '«'
                    } else {
                        // End of text after ". " — still split
                        true
                    };

                    if next_content {
                        let sentence: String = chars[current_start..=i].iter().collect();
                        let s = sentence.trim().to_string();
                        if !s.is_empty() {
                            sentences.push(s);
                        }
                        current_start = i + 1;
                        // Skip whitespace
                        while current_start < len
                            && (chars[current_start] == ' ' || chars[current_start] == '\n')
                        {
                            current_start += 1;
                        }
                        i = current_start;
                        continue;
                    }
                }
            }
            i += 1;
        }

        // Remaining text
        if current_start < len {
            let sentence: String = chars[current_start..].iter().collect();
            let s = sentence.trim().to_string();
            if !s.is_empty() {
                sentences.push(s);
            }
        }
    }

    // If splitting produced nothing meaningful, return the whole text as one sentence
    if sentences.is_empty() && !text.trim().is_empty() {
        sentences.push(text.trim().to_string());
    }

    sentences
}

fn score_sentence_relevance(query: &str, sentence: &str) -> f32 {
    let query_words = tokenize_words(query);
    let sentence_words = tokenize_words(sentence);

    if query_words.is_empty() || sentence_words.is_empty() {
        return 0.0;
    }

    let overlap = query_words.intersection(&sentence_words).count();
    if overlap == 0 {
        return 0.0;
    }

    let score = overlap as f32 / query_words.len() as f32;
    let length_factor = (sentence_words.len() as f32).min(30.0) / 30.0;

    score * (0.7 + 0.3 * length_factor)
}

fn extract_relevant_sentences(
    query: &str,
    chunk_content: &str,
    config: &ContextOptimizationConfig,
) -> String {
    let sentences = split_sentences(chunk_content);

    if sentences.len() <= config.min_sentences_per_chunk {
        return chunk_content.to_string();
    }

    // Score each sentence, keeping original index
    let mut scored: Vec<(usize, f32, &String)> = sentences
        .iter()
        .enumerate()
        .map(|(i, s)| (i, score_sentence_relevance(query, s), s))
        .collect();

    // Sort by score descending to pick top sentences
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Determine how many to keep
    let relevant_count = scored.iter().filter(|(_, score, _)| *score > 0.0).count();
    let keep_count = relevant_count
        .max(config.min_sentences_per_chunk)
        .min(config.max_sentences_per_chunk)
        .min(sentences.len());

    // Take top-k by score
    let mut selected: Vec<(usize, &String)> =
        scored.iter().take(keep_count).map(|(i, _, s)| (*i, *s)).collect();

    // Restore original order
    selected.sort_by_key(|(i, _)| *i);

    // Join with " ... " between non-consecutive sentences
    let mut result = String::new();
    let mut prev_idx: Option<usize> = None;

    for (idx, sentence) in &selected {
        if let Some(prev) = prev_idx {
            if *idx == prev + 1 {
                result.push(' ');
            } else {
                result.push_str(" ... ");
            }
        }
        result.push_str(sentence);
        prev_idx = Some(*idx);
    }

    result
}

fn jaccard_similarity(a: &str, b: &str) -> f32 {
    let words_a = tokenize_words(a);
    let words_b = tokenize_words(b);

    if words_a.is_empty() || words_b.is_empty() {
        return 0.0;
    }

    let intersection = words_a.intersection(&words_b).count() as f32;
    let union = words_a.union(&words_b).count() as f32;

    intersection / union
}

fn remove_redundancy(chunks: &mut Vec<OptimizedChunk>, threshold: f32) {
    let mut to_remove: HashSet<usize> = HashSet::new();

    for i in 0..chunks.len() {
        if to_remove.contains(&i) {
            continue;
        }
        for j in (i + 1)..chunks.len() {
            if to_remove.contains(&j) {
                continue;
            }
            let sim = jaccard_similarity(
                &chunks[i].optimized_content,
                &chunks[j].optimized_content,
            );
            if sim > threshold {
                // Remove the lower-scored chunk
                if chunks[i].score >= chunks[j].score {
                    to_remove.insert(j);
                } else {
                    to_remove.insert(i);
                    break; // i is removed, no need to compare further
                }
            }
        }
    }

    // Remove in reverse order to preserve indices
    let mut indices: Vec<usize> = to_remove.into_iter().collect();
    indices.sort_unstable_by(|a, b| b.cmp(a));
    for idx in indices {
        chunks.remove(idx);
    }
}

fn apply_token_budget(chunks: &mut Vec<OptimizedChunk>, budget: usize) {
    if budget == 0 {
        // 0 = unlimited
        return;
    }

    // Recalculate token counts
    for chunk in chunks.iter_mut() {
        chunk.token_count = estimate_tokens(&chunk.optimized_content);
    }

    let total: usize = chunks.iter().map(|c| c.token_count).sum();
    if total <= budget {
        return;
    }

    // Remove lowest-scored chunks until under budget
    // Chunks are already sorted by score from reranker, but let's sort to be safe
    chunks.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    while chunks.len() > 1 {
        let total: usize = chunks.iter().map(|c| c.token_count).sum();
        if total <= budget {
            break;
        }
        chunks.pop(); // Remove lowest-scored (last after sort)
    }

    // If single chunk still exceeds budget, truncate it
    if let Some(chunk) = chunks.first_mut() {
        if chunk.token_count > budget {
            let target_chars = budget * 3; // Reverse of estimate_tokens
            let content = &chunk.optimized_content;
            if content.len() > target_chars {
                // Truncate at sentence boundary if possible
                let sentences = split_sentences(&content[..target_chars.min(content.len())]);
                if sentences.len() > 1 {
                    // Drop last sentence (might be cut off)
                    let truncated = sentences[..sentences.len() - 1].join(" ");
                    chunk.optimized_content = truncated + "...";
                } else {
                    // Find last word boundary
                    let truncated = &content[..target_chars.min(content.len())];
                    if let Some(last_space) = truncated.rfind(' ') {
                        chunk.optimized_content = content[..last_space].to_string() + "...";
                    } else {
                        // UTF-8 safe truncation
                        let mut end = target_chars.min(content.len());
                        while end > 0 && !content.is_char_boundary(end) {
                            end -= 1;
                        }
                        chunk.optimized_content = content[..end].to_string() + "...";
                    }
                }
                chunk.token_count = estimate_tokens(&chunk.optimized_content);
            }
        }
    }
}

/// Main entry point: optimize search results for LLM context injection.
pub fn optimize_context(
    query: &str,
    chunks: &[KbSearchResultItem],
    config: &ContextOptimizationConfig,
) -> (Vec<OptimizedChunk>, OptimizationTrace) {
    let chunks_before = chunks.len();

    // Convert to OptimizedChunk
    let mut optimized: Vec<OptimizedChunk> = chunks
        .iter()
        .map(|c| {
            let token_count = estimate_tokens(&c.content);
            OptimizedChunk {
                chunk_id: c.chunk_id.clone(),
                document_id: c.document_id.clone(),
                document_name: c.document_name.clone(),
                original_content: c.content.clone(),
                optimized_content: c.content.clone(),
                chunk_index: c.chunk_index,
                score: c.score,
                metadata: c.metadata.clone(),
                token_count,
            }
        })
        .collect();

    let original_tokens: usize = optimized.iter().map(|c| c.token_count).sum();

    if !config.enabled {
        return (
            optimized,
            OptimizationTrace {
                original_tokens,
                after_sentence_extraction: original_tokens,
                after_redundancy_removal: original_tokens,
                final_tokens: original_tokens,
                chunks_before,
                chunks_after: chunks_before,
            },
        );
    }

    // Step 1: Sentence extraction
    if config.sentence_extraction {
        for chunk in &mut optimized {
            chunk.optimized_content =
                extract_relevant_sentences(query, &chunk.original_content, config);
            chunk.token_count = estimate_tokens(&chunk.optimized_content);
        }
    }
    let after_sentence_extraction: usize = optimized.iter().map(|c| c.token_count).sum();

    // Step 2: Redundancy removal
    if config.redundancy_removal {
        remove_redundancy(&mut optimized, config.redundancy_threshold);
    }
    let after_redundancy_removal: usize = optimized.iter().map(|c| c.token_count).sum();

    // Step 3: Token budget
    apply_token_budget(&mut optimized, config.token_budget);
    let final_tokens: usize = optimized.iter().map(|c| c.token_count).sum();

    let chunks_after = optimized.len();

    (
        optimized,
        OptimizationTrace {
            original_tokens,
            after_sentence_extraction,
            after_redundancy_removal,
            final_tokens,
            chunks_before,
            chunks_after,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Sentence splitting --

    #[test]
    fn test_split_sentences_basic() {
        let text = "First sentence. Second sentence. Third sentence.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 3);
    }

    #[test]
    fn test_split_sentences_mixed_punctuation() {
        let text = "What is OAuth? It's an auth protocol. Use it wisely!";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 3);
    }

    #[test]
    fn test_split_sentences_newlines() {
        let text = "First paragraph.\n\nSecond paragraph. With two sentences.";
        let sentences = split_sentences(text);
        assert!(sentences.len() >= 2);
    }

    #[test]
    fn test_split_sentences_russian() {
        let text = "Первое предложение. Второе предложение. Третье предложение.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 3);
    }

    #[test]
    fn test_split_sentences_empty() {
        assert!(split_sentences("").is_empty());
        assert!(split_sentences("   ").is_empty());
    }

    #[test]
    fn test_split_sentences_single() {
        let sentences = split_sentences("Just one sentence without period");
        assert_eq!(sentences.len(), 1);
    }

    // -- Sentence relevance scoring --

    #[test]
    fn test_score_sentence_relevance_high_overlap() {
        let score = score_sentence_relevance(
            "OAuth2 authentication error handling",
            "OAuth2 authentication errors are handled by returning 401 status codes.",
        );
        assert!(score > 0.3);
    }

    #[test]
    fn test_score_sentence_relevance_no_overlap() {
        let score = score_sentence_relevance(
            "OAuth2 authentication error handling",
            "The weather is nice today.",
        );
        assert!(score < 0.01);
    }

    #[test]
    fn test_score_sentence_relevance_partial_overlap() {
        let score = score_sentence_relevance(
            "OAuth2 authentication error handling",
            "Authentication is required for all API endpoints.",
        );
        assert!(score > 0.1);
        assert!(score < 0.8);
    }

    // -- Sentence extraction --

    #[test]
    fn test_extract_relevant_sentences() {
        let chunk = "OAuth2 uses bearer tokens. The weather is sunny. Tokens expire after one hour. Birds can fly.";
        let config = ContextOptimizationConfig::default();
        let extracted = extract_relevant_sentences("OAuth2 bearer tokens expire", chunk, &config);
        assert!(extracted.contains("bearer tokens"));
        assert!(extracted.contains("expire"));
        // Irrelevant sentences should be excluded (we have 2 relevant > min_sentences)
        assert!(!extracted.contains("weather") || !extracted.contains("Birds"));
    }

    #[test]
    fn test_extract_preserves_order() {
        let chunk =
            "Step 1: Configure OAuth. Step 2: Get token. Step 3: Call API. Step 4: Handle errors.";
        let config = ContextOptimizationConfig {
            max_sentences_per_chunk: 2,
            ..Default::default()
        };
        let extracted = extract_relevant_sentences("OAuth token", chunk, &config);
        if extracted.contains("Configure") && extracted.contains("token") {
            let pos_configure = extracted.find("Configure").unwrap();
            let pos_token = extracted.find("token").unwrap();
            assert!(pos_configure < pos_token);
        }
    }

    #[test]
    fn test_extract_min_sentences_when_no_overlap() {
        let chunk = "The sky is blue. Water is wet. Fire is hot.";
        let config = ContextOptimizationConfig {
            min_sentences_per_chunk: 2,
            ..Default::default()
        };
        let extracted =
            extract_relevant_sentences("OAuth authentication", chunk, &config);
        // No overlap, but should keep at least min_sentences
        assert!(!extracted.is_empty());
    }

    #[test]
    fn test_extract_short_chunk_unchanged() {
        let chunk = "One sentence only.";
        let config = ContextOptimizationConfig {
            min_sentences_per_chunk: 2,
            ..Default::default()
        };
        let extracted = extract_relevant_sentences("anything", chunk, &config);
        assert_eq!(extracted, chunk);
    }

    // -- Jaccard similarity --

    #[test]
    fn test_jaccard_similarity_identical() {
        let sim = jaccard_similarity("OAuth authentication tokens", "OAuth authentication tokens");
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_jaccard_similarity_different() {
        let sim = jaccard_similarity("OAuth authentication tokens", "Weather forecast sunny");
        assert!(sim < 0.1);
    }

    #[test]
    fn test_jaccard_similarity_partial() {
        let sim = jaccard_similarity(
            "OAuth authentication uses bearer tokens",
            "OAuth authentication requires valid tokens",
        );
        assert!(sim > 0.3);
        assert!(sim < 0.9);
    }

    #[test]
    fn test_jaccard_similarity_empty() {
        assert_eq!(jaccard_similarity("", "something"), 0.0);
        assert_eq!(jaccard_similarity("something", ""), 0.0);
    }

    // -- Redundancy removal --

    fn test_chunk(id: &str) -> OptimizedChunk {
        OptimizedChunk {
            chunk_id: id.to_string(),
            document_id: "doc1".to_string(),
            document_name: "test.md".to_string(),
            original_content: "test content".to_string(),
            optimized_content: "test content".to_string(),
            chunk_index: 0,
            score: 0.5,
            metadata: serde_json::json!({}),
            token_count: 10,
        }
    }

    #[test]
    fn test_remove_redundancy() {
        let mut chunks = vec![
            OptimizedChunk {
                optimized_content: "OAuth uses bearer tokens for authentication.".to_string(),
                score: 0.9,
                ..test_chunk("c1")
            },
            OptimizedChunk {
                optimized_content:
                    "OAuth uses bearer tokens for authentication and authorization.".to_string(),
                score: 0.7,
                ..test_chunk("c2")
            },
            OptimizedChunk {
                optimized_content: "The API supports REST and GraphQL.".to_string(),
                score: 0.6,
                ..test_chunk("c3")
            },
        ];
        remove_redundancy(&mut chunks, 0.7);
        assert_eq!(chunks.len(), 2);
        assert!(chunks.iter().any(|c| c.chunk_id == "c1"));
        assert!(chunks.iter().any(|c| c.chunk_id == "c3"));
    }

    #[test]
    fn test_remove_redundancy_no_duplicates() {
        let mut chunks = vec![
            OptimizedChunk {
                optimized_content: "OAuth uses bearer tokens.".to_string(),
                score: 0.9,
                ..test_chunk("c1")
            },
            OptimizedChunk {
                optimized_content: "The API supports REST endpoints.".to_string(),
                score: 0.7,
                ..test_chunk("c2")
            },
        ];
        remove_redundancy(&mut chunks, 0.85);
        assert_eq!(chunks.len(), 2);
    }

    // -- Token budget --

    #[test]
    fn test_apply_token_budget_under_limit() {
        let mut chunks = vec![
            OptimizedChunk {
                token_count: 100,
                optimized_content: "a".repeat(300),
                ..test_chunk("c1")
            },
            OptimizedChunk {
                token_count: 100,
                optimized_content: "b".repeat(300),
                ..test_chunk("c2")
            },
        ];
        apply_token_budget(&mut chunks, 4000);
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn test_apply_token_budget_over_limit() {
        let mut chunks = vec![
            OptimizedChunk {
                score: 0.9,
                optimized_content: "a".repeat(6000),
                ..test_chunk("c1")
            },
            OptimizedChunk {
                score: 0.8,
                optimized_content: "b".repeat(6000),
                ..test_chunk("c2")
            },
            OptimizedChunk {
                score: 0.7,
                optimized_content: "c".repeat(6000),
                ..test_chunk("c3")
            },
        ];
        apply_token_budget(&mut chunks, 4000);
        assert!(chunks.len() <= 2);
    }

    #[test]
    fn test_apply_token_budget_zero_means_unlimited() {
        let mut chunks = vec![
            OptimizedChunk {
                token_count: 5000,
                optimized_content: "a".repeat(15000),
                ..test_chunk("c1")
            },
            OptimizedChunk {
                token_count: 5000,
                optimized_content: "b".repeat(15000),
                ..test_chunk("c2")
            },
        ];
        apply_token_budget(&mut chunks, 0);
        assert_eq!(chunks.len(), 2);
    }

    // -- Full pipeline --

    #[test]
    fn test_optimize_context_disabled() {
        let config = ContextOptimizationConfig {
            enabled: false,
            ..Default::default()
        };
        let chunks = vec![KbSearchResultItem {
            chunk_id: "c1".to_string(),
            document_id: "doc1".to_string(),
            document_name: "test.md".to_string(),
            content: "Some content here.".to_string(),
            chunk_index: 0,
            score: 0.9,
            metadata: serde_json::json!({}),
        }];
        let (optimized, trace) = optimize_context("query", &chunks, &config);
        assert_eq!(optimized.len(), chunks.len());
        assert_eq!(trace.original_tokens, trace.final_tokens);
    }

    #[test]
    fn test_optimize_context_full_pipeline() {
        let config = ContextOptimizationConfig::default();
        let chunks = vec![KbSearchResultItem {
            chunk_id: "c1".to_string(),
            document_id: "doc1".to_string(),
            document_name: "test.md".to_string(),
            content: "OAuth uses tokens. The sky is blue. Tokens expire hourly. Birds can fly. Weather is nice today. The sun is shining brightly.".to_string(),
            chunk_index: 0,
            score: 0.9,
            metadata: serde_json::json!({}),
        }];
        let (optimized, trace) = optimize_context("OAuth token expiration", &chunks, &config);
        assert!(trace.final_tokens <= trace.original_tokens);
        assert!(
            optimized[0].optimized_content.len() <= optimized[0].original_content.len()
        );
    }

    #[test]
    fn test_optimize_context_empty_input() {
        let config = ContextOptimizationConfig::default();
        let (optimized, trace) = optimize_context("query", &[], &config);
        assert!(optimized.is_empty());
        assert_eq!(trace.original_tokens, 0);
        assert_eq!(trace.final_tokens, 0);
    }

    // -- tokenize_words --

    #[test]
    fn test_tokenize_words_filters_short() {
        let words = tokenize_words("I am a good programmer");
        assert!(!words.contains("am"));
        assert!(!words.contains("a"));
        assert!(words.contains("good"));
        assert!(words.contains("programmer"));
    }

    #[test]
    fn test_tokenize_words_lowercase() {
        let words = tokenize_words("OAuth TOKENS");
        assert!(words.contains("oauth"));
        assert!(words.contains("tokens"));
    }
}
