// Разбиение длинных текстов на чанки с перекрытием

use regex::Regex;
use std::path::Path;

pub struct Chunk {
    pub text: String,
    pub index: usize,
}

/// Result of document chunking with metadata.
pub struct ChunkResult {
    pub content: String,
    /// Context prefix + "\n\n" + content — used for embedding generation.
    /// None for legacy strategies (tokens/paragraphs/headings).
    pub content_with_context: Option<String>,
    /// Heading hierarchy from document structure, e.g. ["API Guide", "Authentication", "OAuth2"]
    pub heading_hierarchy: Vec<String>,
    pub index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub metadata: serde_json::Value,
}

/// Detected chunking strategy based on document type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChunkingStrategy {
    Markdown,
    Code,
    Html,
    PlainText,
}

/// Configuration for enriched chunking.
pub struct ChunkConfig {
    pub max_chunk_size: usize,  // in tokens, default 512
    pub chunk_overlap: usize,   // in tokens, default 50
    pub min_chunk_size: usize,  // in tokens, default 50
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 512,
            chunk_overlap: 50,
            min_chunk_size: 50,
        }
    }
}

/// Approximate token count. Conservative: ~3 chars per token (handles multilingual).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 2) / 3 // ceil division
}

/// Convert token count to approximate word count.
fn tokens_to_words(tokens: usize) -> usize {
    (tokens * 3 / 4).max(1)
}

// ─── Legacy API (unchanged, used by chat embeddings) ───

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

/// Main entry point for KB document chunking (legacy). Dispatches by strategy.
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

// ─── Enriched API (new) ───

/// Detect the best chunking strategy from file name, MIME type, and content.
pub fn detect_strategy(file_name: &str, mime_type: &str, content: &str) -> ChunkingStrategy {
    // 1. Check file extension
    if let Some(ext) = Path::new(file_name).extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "md" | "mdx" | "markdown" => return ChunkingStrategy::Markdown,
            "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "c" | "cpp" | "h"
            | "hpp" | "cs" | "rb" | "php" | "swift" | "kt" | "scala" | "zig" | "lua"
            | "sh" | "bash" | "zsh" | "ps1" => return ChunkingStrategy::Code,
            "html" | "htm" | "xml" | "xhtml" => return ChunkingStrategy::Html,
            "txt" | "csv" | "log" | "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" => {
                // Fall through to content heuristic for plain formats
            }
            _ => {}
        }
    }

    // 2. Check MIME type
    match mime_type {
        "text/markdown" => return ChunkingStrategy::Markdown,
        "text/html" | "application/xhtml+xml" => return ChunkingStrategy::Html,
        m if m.starts_with("text/x-") || m == "application/javascript"
            || m == "application/typescript" => return ChunkingStrategy::Code,
        _ => {}
    }

    // 3. Content heuristic
    let lines: Vec<&str> = content.lines().take(200).collect();
    let heading_count = lines.iter().filter(|l| {
        let trimmed = l.trim();
        trimmed.starts_with('#') && trimmed.len() > 2
            && trimmed.chars().skip_while(|c| *c == '#').next().map_or(false, |c| c == ' ')
    }).count();
    if heading_count >= 3 {
        return ChunkingStrategy::Markdown;
    }

    let code_re = Regex::new(r"^(pub\s+)?(fn |def |function |class |async def |export |impl |struct |enum |func |type\s+\w+\s+struct)").unwrap();
    let code_count = lines.iter().filter(|l| code_re.is_match(l.trim())).count();
    if code_count >= 3 {
        return ChunkingStrategy::Code;
    }

    ChunkingStrategy::PlainText
}

/// Enriched entry point for KB document chunking.
/// Supports auto-detection, new strategies (markdown/code/html), and min chunk merging.
pub fn chunk_document_enriched(
    text: &str,
    strategy: &str,
    chunk_size: usize,
    overlap: usize,
    min_chunk_size: usize,
    file_name: &str,
    mime_type: &str,
) -> Vec<ChunkResult> {
    if text.trim().is_empty() {
        return vec![];
    }

    let config = ChunkConfig {
        max_chunk_size: chunk_size,
        chunk_overlap: overlap,
        min_chunk_size,
    };

    let mut chunks = match strategy {
        "markdown" => chunk_markdown_recursive(text, file_name, &config, &[], 0, 0),
        "code" => chunk_code(text, file_name, &config),
        "html" => chunk_html(text, file_name, &config),
        "plain" => chunk_plain_text(text, file_name, &config),
        "headings" => chunk_by_headings(text, chunk_size),
        "paragraphs" => chunk_by_paragraphs(text, chunk_size),
        "auto" | "tokens" | _ => {
            // Auto-detect strategy
            let detected = detect_strategy(file_name, mime_type, text);
            match detected {
                ChunkingStrategy::Markdown => chunk_markdown_recursive(text, file_name, &config, &[], 0, 0),
                ChunkingStrategy::Code => chunk_code(text, file_name, &config),
                ChunkingStrategy::Html => chunk_html(text, file_name, &config),
                ChunkingStrategy::PlainText => chunk_plain_text(text, file_name, &config),
            }
        }
    };

    // Post-process: merge small chunks
    if min_chunk_size > 0 {
        chunks = merge_small_chunks(chunks, min_chunk_size);
    }

    // Reindex sequentially
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.index = i;
    }

    chunks
}

// ─── Recursive Markdown Chunker ───

/// Recursively split markdown by heading levels, then paragraphs, then sliding window.
fn chunk_markdown_recursive(
    text: &str,
    file_name: &str,
    config: &ChunkConfig,
    parent_hierarchy: &[String],
    depth: usize,
    base_offset: usize,
) -> Vec<ChunkResult> {
    if text.trim().is_empty() {
        return vec![];
    }

    let tokens = estimate_tokens(text);

    // For small text, extract heading hierarchy even if it fits in one chunk
    if tokens <= config.max_chunk_size {
        // Try to extract heading from the text for hierarchy enrichment
        let mut hierarchy = parent_hierarchy.to_vec();
        if hierarchy.is_empty() {
            let heading_re = Regex::new(r"(?m)^(#{1,6})\s+(.+)$").unwrap();
            if let Some(caps) = heading_re.captures(text) {
                hierarchy.push(caps[2].trim().to_string());
            }
        }
        let context_prefix = build_context_prefix(file_name, &hierarchy);
        return vec![ChunkResult {
            content: text.trim().to_string(),
            content_with_context: Some(build_content_with_context(&context_prefix, text.trim())),
            heading_hierarchy: hierarchy.clone(),
            index: 0,
            start_offset: base_offset,
            end_offset: base_offset + text.len(),
            metadata: serde_json::json!({
                "strategy": "markdown",
                "section_title": hierarchy.last().cloned().unwrap_or_default(),
                "heading_hierarchy": &hierarchy,
            }),
        }];
    }

    // Try splitting by heading levels 1-4 (# through ####)
    // Start from the current depth and try each level until we find a split
    for heading_level in (depth + 1)..=4 {
        let sections = split_by_heading_level(text, heading_level);

        if sections.len() > 1 {
            let mut results = Vec::new();
            for section in &sections {
                let mut hierarchy = parent_hierarchy.to_vec();
                if let Some(ref heading) = section.heading {
                    hierarchy.push(heading.clone());
                }
                let sub_chunks = chunk_markdown_recursive(
                    &section.text, file_name, config,
                    &hierarchy, heading_level, // continue from this heading level
                    base_offset + section.start_offset,
                );
                results.extend(sub_chunks);
            }
            return results;
        }
    }

    // Depth 4: split by paragraphs
    if depth <= 4 {
        let paragraphs = split_by_paragraphs(text);
        if paragraphs.len() > 1 {
            let mut results = Vec::new();
            let mut current_content = String::new();
            let mut current_start = 0usize;

            for para in &paragraphs {
                let combined_tokens = estimate_tokens(&current_content) + estimate_tokens(&para.text);
                if !current_content.is_empty() && combined_tokens > config.max_chunk_size {
                    // Flush current
                    let context_prefix = build_context_prefix(file_name, parent_hierarchy);
                    let trimmed = current_content.trim().to_string();
                    if !trimmed.is_empty() {
                        results.push(ChunkResult {
                            content: trimmed.clone(),
                            content_with_context: Some(build_content_with_context(&context_prefix, &trimmed)),
                            heading_hierarchy: parent_hierarchy.to_vec(),
                            index: 0,
                            start_offset: base_offset + current_start,
                            end_offset: base_offset + para.start_offset,
                            metadata: serde_json::json!({
                                "strategy": "markdown",
                                "section_title": parent_hierarchy.last().cloned().unwrap_or_default(),
                                "heading_hierarchy": parent_hierarchy,
                            }),
                        });
                    }
                    current_content = String::new();
                    current_start = para.start_offset;
                }
                if current_content.is_empty() {
                    current_start = para.start_offset;
                } else {
                    current_content.push_str("\n\n");
                }
                current_content.push_str(&para.text);
            }

            // Flush remaining
            if !current_content.trim().is_empty() {
                let context_prefix = build_context_prefix(file_name, parent_hierarchy);
                let trimmed = current_content.trim().to_string();
                // If still too large, sliding window
                if estimate_tokens(&trimmed) > config.max_chunk_size {
                    let sub = sliding_window_chunks(
                        &trimmed, file_name, config, parent_hierarchy,
                        base_offset + current_start,
                    );
                    results.extend(sub);
                } else {
                    results.push(ChunkResult {
                        content: trimmed.clone(),
                        content_with_context: Some(build_content_with_context(&context_prefix, &trimmed)),
                        heading_hierarchy: parent_hierarchy.to_vec(),
                        index: 0,
                        start_offset: base_offset + current_start,
                        end_offset: base_offset + text.len(),
                        metadata: serde_json::json!({
                            "strategy": "markdown",
                            "section_title": parent_hierarchy.last().cloned().unwrap_or_default(),
                            "heading_hierarchy": parent_hierarchy,
                        }),
                    });
                }
            }

            if !results.is_empty() {
                return results;
            }
        }
    }

    // Deepest level: sliding window
    sliding_window_chunks(text, file_name, config, parent_hierarchy, base_offset)
}

/// Sliding window chunker with overlap and sentence-boundary preference.
fn sliding_window_chunks(
    text: &str,
    file_name: &str,
    config: &ChunkConfig,
    hierarchy: &[String],
    base_offset: usize,
) -> Vec<ChunkResult> {
    let max_words = tokens_to_words(config.max_chunk_size);
    let overlap_words = tokens_to_words(config.chunk_overlap);
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return vec![];
    }

    let word_offsets = find_word_offsets(text);
    let step = max_words.saturating_sub(overlap_words).max(1);
    let context_prefix = build_context_prefix(file_name, hierarchy);
    let mut results = Vec::new();
    let mut start = 0;

    while start < words.len() {
        let end = (start + max_words).min(words.len());
        let content = words[start..end].join(" ");
        let start_byte = word_offsets.get(start).map(|w| w.0).unwrap_or(0);
        let end_byte = word_offsets.get(end - 1).map(|w| w.1).unwrap_or(text.len());

        results.push(ChunkResult {
            content: content.clone(),
            content_with_context: Some(build_content_with_context(&context_prefix, &content)),
            heading_hierarchy: hierarchy.to_vec(),
            index: 0,
            start_offset: base_offset + start_byte,
            end_offset: base_offset + end_byte,
            metadata: serde_json::json!({
                "strategy": "markdown",
                "section_title": hierarchy.last().cloned().unwrap_or_default(),
                "heading_hierarchy": hierarchy,
            }),
        });

        if end >= words.len() {
            break;
        }
        start += step;
    }
    results
}

// ─── Code Chunker ───

/// Chunk source code by function/class boundaries using regex patterns.
fn chunk_code(text: &str, file_name: &str, config: &ChunkConfig) -> Vec<ChunkResult> {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let pattern = get_code_split_pattern(&ext);
    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return chunk_plain_text(text, file_name, config),
    };

    let mut split_points: Vec<usize> = re.find_iter(text)
        .map(|m| m.start())
        .collect();

    if split_points.is_empty() {
        return chunk_plain_text(text, file_name, config);
    }

    // Ensure we start from 0
    if split_points[0] != 0 {
        split_points.insert(0, 0);
    }

    let mut results = Vec::new();
    for i in 0..split_points.len() {
        let start = split_points[i];
        let end = if i + 1 < split_points.len() { split_points[i + 1] } else { text.len() };
        let section = text[start..end].trim();

        if section.is_empty() {
            continue;
        }

        let symbol_name = extract_symbol_name(section, &ext);
        let section_title = if i == 0 && split_points.len() > 1 && start == 0 && !re.is_match(section.lines().next().unwrap_or("")) {
            "Preamble".to_string()
        } else {
            symbol_name.clone()
        };

        let hierarchy = vec![file_name.to_string(), section_title.clone()];
        let context_prefix = format!("{} > {}", file_name, section_title);

        if estimate_tokens(section) <= config.max_chunk_size {
            results.push(ChunkResult {
                content: section.to_string(),
                content_with_context: Some(build_content_with_context(&context_prefix, section)),
                heading_hierarchy: hierarchy,
                index: 0,
                start_offset: start,
                end_offset: end,
                metadata: serde_json::json!({
                    "strategy": "code",
                    "language": &ext,
                    "symbol": &section_title,
                }),
            });
        } else {
            // Sub-split large functions with sliding window
            let sub = sliding_window_chunks_code(section, file_name, config, &section_title, &ext, start);
            results.extend(sub);
        }
    }

    results
}

/// Sliding window for code that's too large.
fn sliding_window_chunks_code(
    text: &str,
    file_name: &str,
    config: &ChunkConfig,
    symbol: &str,
    lang: &str,
    base_offset: usize,
) -> Vec<ChunkResult> {
    let max_words = tokens_to_words(config.max_chunk_size);
    let overlap_words = tokens_to_words(config.chunk_overlap);
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return vec![];
    }

    let word_offsets = find_word_offsets(text);
    let step = max_words.saturating_sub(overlap_words).max(1);
    let hierarchy = vec![file_name.to_string(), symbol.to_string()];
    let context_prefix = format!("{} > {}", file_name, symbol);
    let mut results = Vec::new();
    let mut start = 0;

    while start < words.len() {
        let end = (start + max_words).min(words.len());
        let content = words[start..end].join(" ");
        let start_byte = word_offsets.get(start).map(|w| w.0).unwrap_or(0);
        let end_byte = word_offsets.get(end - 1).map(|w| w.1).unwrap_or(text.len());

        results.push(ChunkResult {
            content: content.clone(),
            content_with_context: Some(build_content_with_context(&context_prefix, &content)),
            heading_hierarchy: hierarchy.clone(),
            index: 0,
            start_offset: base_offset + start_byte,
            end_offset: base_offset + end_byte,
            metadata: serde_json::json!({
                "strategy": "code",
                "language": lang,
                "symbol": symbol,
            }),
        });

        if end >= words.len() {
            break;
        }
        start += step;
    }
    results
}

/// Get regex pattern for splitting code by language.
fn get_code_split_pattern(ext: &str) -> String {
    match ext {
        "rs" => r"(?m)^\s*(pub\s+)?(async\s+)?(fn|struct|enum|impl|trait|mod|const\s+[A-Z]|static)\s+".to_string(),
        "py" => r"(?m)^(class|def|async\s+def)\s+".to_string(),
        "ts" | "tsx" | "js" | "jsx" => r"(?m)^(export\s+)?(default\s+)?(async\s+)?(function\*?|class|interface|type|enum)\s+".to_string(),
        "go" => r"(?m)^(func|type)\s+".to_string(),
        "java" | "cs" | "kt" => r"(?m)^\s*(public|private|protected|internal)?\s*(static\s+)?(abstract\s+)?(class|interface|enum|record|fun|void|int|String|boolean|long|double)\s+".to_string(),
        "c" | "cpp" | "h" | "hpp" => r"(?m)^(\w[\w\s\*]*\s+)?\w+\s*\([^)]*\)\s*\{".to_string(),
        "rb" => r"(?m)^(class|def|module)\s+".to_string(),
        "php" => r"(?m)^(class|function|public\s+function|private\s+function|protected\s+function)\s+".to_string(),
        "swift" => r"(?m)^(func|class|struct|enum|protocol|extension)\s+".to_string(),
        _ => r"(?m)^(function|def|class|fn|pub\s+fn|export\s+function|export\s+class)\s+".to_string(),
    }
}

/// Extract symbol name from the first line of a code section.
fn extract_symbol_name(section: &str, ext: &str) -> String {
    let first_line = section.lines().next().unwrap_or("").trim();

    // Try to extract function/class/struct name
    let name_re = match ext {
        "rs" => Regex::new(r"(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|impl|trait|mod)\s+(\w+)"),
        "py" => Regex::new(r"(?:class|def|async\s+def)\s+(\w+)"),
        "ts" | "tsx" | "js" | "jsx" => Regex::new(r"(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:function\*?|class|interface|type|enum)\s+(\w+)"),
        "go" => Regex::new(r"(?:func|type)\s+(?:\([^)]+\)\s+)?(\w+)"),
        _ => Regex::new(r"(?:function|def|class|fn|pub\s+fn)\s+(\w+)"),
    };

    if let Ok(re) = name_re {
        if let Some(caps) = re.captures(first_line) {
            if let Some(name) = caps.get(1) {
                let keyword = first_line.split_whitespace()
                    .find(|w| ["fn", "def", "function", "class", "struct", "enum", "impl", "trait", "mod", "interface", "type", "func"].contains(w))
                    .unwrap_or("");
                return if keyword.is_empty() {
                    name.as_str().to_string()
                } else {
                    format!("{} {}", keyword, name.as_str())
                };
            }
        }
    }

    // Fallback: first meaningful token
    first_line.chars().take(60).collect::<String>()
}

// ─── HTML Chunker ───

/// Chunk HTML by converting headings to markdown, stripping tags, then using markdown chunker.
fn chunk_html(text: &str, file_name: &str, config: &ChunkConfig) -> Vec<ChunkResult> {
    let converted = html_to_markdown_like(text);
    let mut chunks = chunk_markdown_recursive(&converted, file_name, config, &[], 0, 0);

    // Override strategy in metadata
    for chunk in &mut chunks {
        if let Some(obj) = chunk.metadata.as_object_mut() {
            obj.insert("strategy".to_string(), serde_json::json!("html"));
        }
    }

    // Note: offsets refer to converted text, not original HTML.
    // For KB purposes this is acceptable since we store chunk.content, not original.
    chunks
}

/// Convert HTML headings to markdown format and strip other tags.
fn html_to_markdown_like(html: &str) -> String {
    let mut result = html.to_string();

    // Remove script, style, noscript blocks (no backreferences — match each tag separately)
    for tag in &["script", "style", "noscript"] {
        let re = Regex::new(&format!(r"(?si)<{tag}[^>]*>.*?</{tag}>")).unwrap();
        result = re.replace_all(&result, "").to_string();
    }

    // Convert headings: <h1>Title</h1> → # Title
    for level in 1..=6usize {
        let hashes = "#".repeat(level);
        let heading_re = Regex::new(&format!(r"(?si)<h{level}[^>]*>(.*?)</h{level}>")).unwrap();
        result = heading_re.replace_all(&result, |caps: &regex::Captures| {
            let inner = strip_inline_tags(&caps[1]);
            format!("\n\n{} {}\n\n", hashes, inner.trim())
        }).to_string();
    }

    // Convert block elements
    let p_re = Regex::new(r"(?si)<p[^>]*>(.*?)</p>").unwrap();
    result = p_re.replace_all(&result, |caps: &regex::Captures| {
        format!("\n\n{}\n\n", &caps[1])
    }).to_string();

    let li_re = Regex::new(r"(?si)<li[^>]*>(.*?)</li>").unwrap();
    result = li_re.replace_all(&result, |caps: &regex::Captures| {
        format!("\n- {}", &caps[1].trim())
    }).to_string();

    let br_re = Regex::new(r"<br\s*/?>").unwrap();
    result = br_re.replace_all(&result, "\n").to_string();

    // Strip remaining tags
    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    result = tag_re.replace_all(&result, "").to_string();

    // Decode common HTML entities
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");
    result = result.replace("&#39;", "'");
    result = result.replace("&nbsp;", " ");

    // Normalize whitespace: collapse multiple blank lines
    let multi_newline_re = Regex::new(r"\n{3,}").unwrap();
    result = multi_newline_re.replace_all(&result, "\n\n").to_string();

    result.trim().to_string()
}

/// Strip inline HTML tags from text (keep content).
fn strip_inline_tags(text: &str) -> String {
    let re = Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(text, "").to_string()
}

// ─── Plain Text Chunker ───

/// Enhanced plain text chunker: paragraph-based merging with sliding window fallback.
fn chunk_plain_text(text: &str, file_name: &str, config: &ChunkConfig) -> Vec<ChunkResult> {
    let paragraphs = split_by_paragraphs(text);
    let context_prefix = file_name.to_string();
    let mut results = Vec::new();
    let mut current_content = String::new();
    let mut current_start = 0usize;

    for para in &paragraphs {
        let combined_tokens = estimate_tokens(&current_content) + estimate_tokens(&para.text);

        if !current_content.is_empty() && combined_tokens > config.max_chunk_size {
            // Flush
            let trimmed = current_content.trim().to_string();
            if !trimmed.is_empty() {
                if estimate_tokens(&trimmed) > config.max_chunk_size {
                    // Still too large — sliding window
                    let sub = sliding_window_chunks(&trimmed, file_name, config, &[], current_start);
                    for mut c in sub {
                        c.metadata = serde_json::json!({"strategy": "plain"});
                        results.push(c);
                    }
                } else {
                    results.push(ChunkResult {
                        content: trimmed.clone(),
                        content_with_context: Some(build_content_with_context(&context_prefix, &trimmed)),
                        heading_hierarchy: vec![],
                        index: 0,
                        start_offset: current_start,
                        end_offset: para.start_offset,
                        metadata: serde_json::json!({"strategy": "plain"}),
                    });
                }
            }
            current_content = String::new();
            current_start = para.start_offset;
        }

        if current_content.is_empty() {
            current_start = para.start_offset;
        } else {
            current_content.push_str("\n\n");
        }
        current_content.push_str(&para.text);
    }

    // Flush remaining
    if !current_content.trim().is_empty() {
        let trimmed = current_content.trim().to_string();
        if estimate_tokens(&trimmed) > config.max_chunk_size {
            let sub = sliding_window_chunks(&trimmed, file_name, config, &[], current_start);
            for mut c in sub {
                c.metadata = serde_json::json!({"strategy": "plain"});
                results.push(c);
            }
        } else {
            results.push(ChunkResult {
                content: trimmed.clone(),
                content_with_context: Some(build_content_with_context(&context_prefix, &trimmed)),
                heading_hierarchy: vec![],
                index: 0,
                start_offset: current_start,
                end_offset: text.len(),
                metadata: serde_json::json!({"strategy": "plain"}),
            });
        }
    }

    results
}

// ─── Merge Small Chunks ───

/// Merge consecutive chunks smaller than min_chunk_size with their neighbor.
fn merge_small_chunks(mut chunks: Vec<ChunkResult>, min_chunk_size: usize) -> Vec<ChunkResult> {
    if chunks.len() <= 1 {
        return chunks;
    }

    let mut merged: Vec<ChunkResult> = Vec::new();

    while let Some(chunk) = chunks.first() {
        if estimate_tokens(&chunk.content) >= min_chunk_size || merged.is_empty() {
            merged.push(chunks.remove(0));
        } else {
            // Merge with previous chunk
            let small = chunks.remove(0);
            if let Some(prev) = merged.last_mut() {
                prev.content.push_str("\n\n");
                prev.content.push_str(&small.content);
                prev.end_offset = small.end_offset;
                // Rebuild content_with_context
                if prev.content_with_context.is_some() {
                    let prefix = if !prev.heading_hierarchy.is_empty() {
                        prev.heading_hierarchy.join(" > ")
                    } else {
                        String::new()
                    };
                    prev.content_with_context = Some(build_content_with_context(&prefix, &prev.content));
                }
            }
        }
    }

    merged
}

// ─── Legacy Chunkers (preserved for backward compat) ───

/// Chunk by approximate token count using word-based splitting.
fn chunk_by_tokens(text: &str, chunk_size: usize, overlap: usize) -> Vec<ChunkResult> {
    let max_words = tokens_to_words(chunk_size);
    let overlap_words = (overlap * 3 / 4).max(0);
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }
    if words.len() <= max_words {
        return vec![ChunkResult {
            content: text.to_string(),
            content_with_context: None,
            heading_hierarchy: vec![],
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

    let word_offsets: Vec<(usize, usize)> = find_word_offsets(text);

    while start < words.len() {
        let end = (start + max_words).min(words.len());
        let content = words[start..end].join(" ");
        let start_offset = word_offsets.get(start).map(|w| w.0).unwrap_or(0);
        let end_offset = word_offsets.get(end - 1).map(|w| w.1).unwrap_or(text.len());

        results.push(ChunkResult {
            content,
            content_with_context: None,
            heading_hierarchy: vec![],
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

/// Chunk by markdown headings (# ## ### etc.) — legacy strategy.
fn chunk_by_headings(text: &str, max_chunk_size: usize) -> Vec<ChunkResult> {
    let max_words = tokens_to_words(max_chunk_size);
    let heading_re = Regex::new(r"(?m)^(#{1,6})\s+(.+)$").unwrap();

    let mut sections: Vec<(String, usize, usize)> = Vec::new();
    let mut last_start = 0;
    let mut last_heading = String::new();

    for m in heading_re.find_iter(text) {
        if m.start() > last_start || !last_heading.is_empty() {
            sections.push((last_heading.clone(), last_start, m.start()));
        }
        last_heading = m.as_str().to_string();
        last_start = m.start();
    }
    if last_start < text.len() {
        sections.push((last_heading, last_start, text.len()));
    }

    if sections.is_empty() {
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
                content_with_context: None,
                heading_hierarchy: vec![],
                index,
                start_offset: *start,
                end_offset: *end,
                metadata: serde_json::json!({"strategy": "headings", "heading": heading}),
            });
            index += 1;
        } else {
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

/// Chunk by paragraphs (double newline separated), merging short ones — legacy strategy.
fn chunk_by_paragraphs(text: &str, max_chunk_size: usize) -> Vec<ChunkResult> {
    let max_words = tokens_to_words(max_chunk_size);
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
            offset += para.len() + 2;
            continue;
        }
        let para_words = trimmed.split_whitespace().count();

        if current_words > 0 && current_words + para_words > max_words {
            results.push(ChunkResult {
                content: current_content.trim().to_string(),
                content_with_context: None,
                heading_hierarchy: vec![],
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

        offset += para.len() + 2;
    }

    if !current_content.is_empty() {
        results.push(ChunkResult {
            content: current_content.trim().to_string(),
            content_with_context: None,
            heading_hierarchy: vec![],
            index,
            start_offset: current_start,
            end_offset: text.len(),
            metadata: serde_json::json!({"strategy": "paragraphs", "paragraph": para_num}),
        });
    }

    if results.is_empty() && !text.trim().is_empty() {
        results.push(ChunkResult {
            content: text.trim().to_string(),
            content_with_context: None,
            heading_hierarchy: vec![],
            index: 0,
            start_offset: 0,
            end_offset: text.len(),
            metadata: serde_json::json!({"strategy": "paragraphs", "paragraph": 0}),
        });
    }

    results
}

// ─── Helpers ───

struct HeadingSection {
    heading: Option<String>,
    text: String,
    start_offset: usize,
}

struct Paragraph {
    text: String,
    start_offset: usize,
}

/// Split text by a specific heading level (1=# , 2=## , etc.)
fn split_by_heading_level(text: &str, level: usize) -> Vec<HeadingSection> {
    let pattern = format!(r"(?m)^{} +(.+)$", "#".repeat(level));
    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return vec![HeadingSection {
            heading: None,
            text: text.to_string(),
            start_offset: 0,
        }],
    };

    let mut sections = Vec::new();
    let mut last_start = 0;
    let mut last_heading: Option<String> = None;

    for m in re.find_iter(text) {
        // Push previous section
        if m.start() > last_start || last_heading.is_some() {
            let section_text = &text[last_start..m.start()];
            if !section_text.trim().is_empty() {
                sections.push(HeadingSection {
                    heading: last_heading.clone(),
                    text: section_text.to_string(),
                    start_offset: last_start,
                });
            }
        }
        // Extract heading text (without the # prefix)
        let caps = re.captures(m.as_str());
        last_heading = caps.and_then(|c| c.get(1)).map(|m| m.as_str().trim().to_string());
        last_start = m.start();
    }

    // Last section
    if last_start < text.len() {
        let section_text = &text[last_start..];
        if !section_text.trim().is_empty() {
            sections.push(HeadingSection {
                heading: last_heading,
                text: section_text.to_string(),
                start_offset: last_start,
            });
        }
    }

    if sections.is_empty() && !text.trim().is_empty() {
        sections.push(HeadingSection {
            heading: None,
            text: text.to_string(),
            start_offset: 0,
        });
    }

    sections
}

/// Split text into paragraphs by double newline.
fn split_by_paragraphs(text: &str) -> Vec<Paragraph> {
    let mut paragraphs = Vec::new();
    let mut offset = 0;

    for part in text.split("\n\n") {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            paragraphs.push(Paragraph {
                text: trimmed.to_string(),
                start_offset: offset,
            });
        }
        offset += part.len() + 2; // +2 for \n\n
    }

    paragraphs
}

/// Build context prefix from file name and heading hierarchy.
fn build_context_prefix(file_name: &str, hierarchy: &[String]) -> String {
    if hierarchy.is_empty() {
        return file_name.to_string();
    }
    let h = hierarchy.join(" > ");
    if file_name.is_empty() {
        h
    } else {
        format!("{} > {}", file_name, h)
    }
}

/// Build content_with_context: prefix + "\n\n" + content.
fn build_content_with_context(prefix: &str, content: &str) -> String {
    if prefix.is_empty() {
        content.to_string()
    } else {
        // Cap prefix at 200 chars to avoid wasting embedding dimensions
        let capped_prefix = if prefix.len() > 200 {
            let mut end = 200;
            while end > 0 && !prefix.is_char_boundary(end) {
                end -= 1;
            }
            &prefix[..end]
        } else {
            prefix
        };
        format!("{}\n\n{}", capped_prefix, content)
    }
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

    // ─── Legacy tests ───

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
        assert_eq!(chunks.len(), 1);
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
            assert!(!c.text.is_empty());
            assert!(c.text.is_char_boundary(c.text.len()));
        }
    }

    #[test]
    fn chunk_very_long_line_no_separators() {
        let text = "a".repeat(10000);
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
        assert_eq!(offsets[0].0, 0);
        assert_eq!(offsets[0].1, 12);
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
        let meta = &chunks[0].metadata;
        assert_eq!(meta["strategy"], "tokens");
    }

    #[test]
    fn chunk_headings_no_headings_falls_back() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = chunk_document(text, "headings", 1000, 0);
        assert!(!chunks.is_empty());
        let all_content: String = chunks.iter().map(|c| c.content.clone()).collect();
        assert!(all_content.contains("First paragraph"));
        assert!(all_content.contains("Third paragraph"));
    }

    // ─── Type detection tests ───

    #[test]
    fn test_detect_strategy_by_extension() {
        assert_eq!(detect_strategy("readme.md", "text/plain", ""), ChunkingStrategy::Markdown);
        assert_eq!(detect_strategy("main.rs", "text/plain", ""), ChunkingStrategy::Code);
        assert_eq!(detect_strategy("index.html", "text/html", ""), ChunkingStrategy::Html);
        assert_eq!(detect_strategy("data.txt", "text/plain", ""), ChunkingStrategy::PlainText);
        assert_eq!(detect_strategy("notes.csv", "text/csv", ""), ChunkingStrategy::PlainText);
        assert_eq!(detect_strategy("app.tsx", "text/plain", ""), ChunkingStrategy::Code);
        assert_eq!(detect_strategy("doc.mdx", "text/plain", ""), ChunkingStrategy::Markdown);
    }

    #[test]
    fn test_detect_strategy_by_mime() {
        assert_eq!(detect_strategy("unknown", "text/markdown", ""), ChunkingStrategy::Markdown);
        assert_eq!(detect_strategy("unknown", "text/html", ""), ChunkingStrategy::Html);
        assert_eq!(detect_strategy("unknown", "text/x-python", ""), ChunkingStrategy::Code);
    }

    #[test]
    fn test_detect_strategy_by_content_heuristic() {
        let md_content = "# Title\n## Section\n### Subsection\nSome text";
        assert_eq!(detect_strategy("unknown.txt", "text/plain", md_content), ChunkingStrategy::Markdown);

        let code_content = "fn main() {\n}\n\nfn helper() {\n}\n\npub fn process() {\n}";
        assert_eq!(detect_strategy("unknown.txt", "text/plain", code_content), ChunkingStrategy::Code);

        let plain_content = "Just some regular text.\nNothing special here.";
        assert_eq!(detect_strategy("unknown.txt", "text/plain", plain_content), ChunkingStrategy::PlainText);
    }

    // ─── Markdown chunker tests ───

    #[test]
    fn test_markdown_chunker_simple() {
        let content = "# Title\n\nIntro paragraph.\n\n## Section 1\n\nContent of section 1.\n\n## Section 2\n\nContent of section 2.";
        let chunks = chunk_document_enriched(content, "markdown", 500, 50, 0, "doc.md", "text/markdown");

        assert!(chunks.len() >= 1);
        // Should have heading hierarchy
        let has_hierarchy = chunks.iter().any(|c| !c.heading_hierarchy.is_empty());
        assert!(has_hierarchy, "At least one chunk should have heading hierarchy");
    }

    #[test]
    fn test_markdown_chunker_recursive_split() {
        let large_section = "paragraph word. ".repeat(200);
        let content = format!("# Doc\n\n## Section 1\n\n{}\n\n## Section 2\n\nShort.", large_section);
        let chunks = chunk_document_enriched(&content, "markdown", 200, 20, 0, "doc.md", "text/markdown");

        assert!(chunks.len() > 2, "Large section should be split: got {} chunks", chunks.len());
        for chunk in &chunks {
            let tokens = estimate_tokens(&chunk.content);
            // Token estimation is approximate (word-based splitting vs char-based estimation)
            // Allow ~2x due to mismatch between word counting and char/3 estimation
            assert!(tokens <= 450, "Chunk too large: {} tokens", tokens);
        }
    }

    #[test]
    fn test_context_prefix_construction() {
        let content = "# API Guide\n\n## Authentication\n\n### OAuth2\n\nUse bearer tokens for authentication.";
        let chunks = chunk_document_enriched(content, "markdown", 500, 50, 0, "api.md", "text/markdown");

        let oauth_chunk = chunks.iter().find(|c| c.content.contains("bearer tokens"));
        assert!(oauth_chunk.is_some(), "Should find chunk with bearer tokens");
        let chunk = oauth_chunk.unwrap();

        assert!(!chunk.heading_hierarchy.is_empty(), "Should have heading hierarchy");
        assert!(chunk.content_with_context.is_some(), "Should have content_with_context");
        let cwc = chunk.content_with_context.as_ref().unwrap();
        assert!(cwc.contains("OAuth2") || cwc.contains("Authentication"),
            "content_with_context should contain hierarchy: {}", cwc);
    }

    // ─── Code chunker tests ───

    #[test]
    fn test_code_chunker_rust() {
        let content = "use std::io;\n\nfn main() {\n    println!(\"hello\");\n}\n\npub fn helper(x: i32) -> i32 {\n    x + 1\n}";
        let chunks = chunk_document_enriched(content, "code", 500, 50, 0, "main.rs", "text/x-rust");

        assert!(chunks.len() >= 2, "Should have at least 2 chunks (preamble + functions), got {}", chunks.len());
        let has_code_strategy = chunks.iter().all(|c| c.metadata["strategy"] == "code");
        assert!(has_code_strategy);
    }

    #[test]
    fn test_code_chunker_python() {
        let content = "import os\n\ndef hello():\n    print('hello')\n\nclass MyClass:\n    def method(self):\n        pass";
        let chunks = chunk_document_enriched(content, "code", 500, 50, 0, "main.py", "text/x-python");

        assert!(chunks.len() >= 2, "Should have at least 2 chunks, got {}", chunks.len());
    }

    #[test]
    fn test_code_chunker_javascript() {
        let content = "import React from 'react';\n\nexport function App() {\n  return <div/>;\n}\n\nexport class Helper {\n  run() {}\n}";
        let chunks = chunk_document_enriched(content, "code", 500, 50, 0, "app.tsx", "text/plain");

        assert!(chunks.len() >= 2, "Should have at least 2 chunks, got {}", chunks.len());
    }

    // ─── HTML chunker tests ───

    #[test]
    fn test_html_chunker() {
        let content = "<html><body><h1>Title</h1><p>Intro.</p><h2>Section</h2><p>Content here.</p></body></html>";
        let chunks = chunk_document_enriched(content, "html", 500, 50, 0, "page.html", "text/html");

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.content.contains("<h1>"), "Should strip HTML tags");
            assert!(!chunk.content.contains("<p>"), "Should strip HTML tags");
        }
        let all_content: String = chunks.iter().map(|c| c.content.clone()).collect::<Vec<_>>().join(" ");
        assert!(all_content.contains("Title"), "Should preserve heading text");
        assert!(all_content.contains("Content here"), "Should preserve paragraph text");
    }

    #[test]
    fn test_html_chunker_with_entities() {
        let content = "<p>Hello &amp; world &lt;test&gt;</p>";
        let chunks = chunk_document_enriched(content, "html", 500, 50, 0, "page.html", "text/html");

        assert!(!chunks.is_empty());
        assert!(chunks[0].content.contains("Hello & world <test>"));
    }

    // ─── Plain text chunker tests ───

    #[test]
    fn test_plain_text_chunker_paragraph_merge() {
        let content = "Short paragraph 1.\n\nShort paragraph 2.\n\nShort paragraph 3.";
        let chunks = chunk_document_enriched(content, "plain", 500, 50, 0, "notes.txt", "text/plain");

        assert_eq!(chunks.len(), 1, "Short paragraphs should be merged");
    }

    #[test]
    fn test_plain_text_large_paragraphs() {
        let large = "word ".repeat(500);
        let content = format!("{}\n\n{}", large, large);
        let chunks = chunk_document_enriched(&content, "plain", 100, 10, 0, "data.txt", "text/plain");

        assert!(chunks.len() > 2, "Large paragraphs should be split");
    }

    // ─── Auto-detection tests ───

    #[test]
    fn test_auto_detect_markdown() {
        let content = "# Title\n\n## Section\n\nSome content here.";
        let chunks = chunk_document_enriched(content, "auto", 500, 50, 0, "doc.md", "text/markdown");

        assert!(!chunks.is_empty());
        let strategy = chunks[0].metadata["strategy"].as_str().unwrap_or("");
        assert_eq!(strategy, "markdown");
    }

    #[test]
    fn test_auto_detect_code() {
        let content = "fn main() {\n    println!(\"hello\");\n}\n\npub fn helper() {}";
        let chunks = chunk_document_enriched(content, "auto", 500, 50, 0, "main.rs", "text/plain");

        assert!(!chunks.is_empty());
        let strategy = chunks[0].metadata["strategy"].as_str().unwrap_or("");
        assert_eq!(strategy, "code");
    }

    // ─── Min chunk size tests ───

    #[test]
    fn test_min_chunk_size_filtering() {
        let content = "# Title\n\nA.\n\n## Section\n\nReal content here with enough text to be meaningful and form a proper chunk.";
        let chunks = chunk_document_enriched(content, "markdown", 500, 50, 30, "doc.md", "text/markdown");

        // After merging, no chunk should be tiny
        for chunk in &chunks {
            let tokens = estimate_tokens(&chunk.content);
            // Either above min size, or everything was merged into one
            assert!(tokens >= 10 || chunks.len() == 1,
                "Tiny chunk ({} tokens): '{}'", tokens, chunk.content);
        }
    }

    // ─── content_with_context tests ───

    #[test]
    fn test_content_with_context_differs_from_content() {
        let content = "# Guide\n\n## Auth\n\nUse tokens for auth.";
        let chunks = chunk_document_enriched(content, "markdown", 500, 50, 0, "guide.md", "text/markdown");

        for chunk in &chunks {
            if let Some(ref cwc) = chunk.content_with_context {
                if !chunk.heading_hierarchy.is_empty() {
                    assert_ne!(&chunk.content, cwc,
                        "content_with_context should differ from content when hierarchy exists");
                    assert!(cwc.len() > chunk.content.len(),
                        "content_with_context should be longer than content");
                }
            }
        }
    }

    // ─── UTF-8 safety tests ───

    #[test]
    fn test_utf8_safe_splitting() {
        let content = "# Руководство\n\n## Аутентификация\n\nИспользуйте токены для аутентификации. Это важный шаг в процессе настройки безопасности.";
        let chunks = chunk_document_enriched(content, "markdown", 20, 5, 0, "guide.md", "text/markdown");

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            // All chunks are valid UTF-8 (String type guarantees, but verify no panics)
            assert!(!chunk.content.is_empty());
        }
    }

    // ─── Edge case tests ───

    #[test]
    fn test_empty_document() {
        let chunks = chunk_document_enriched("", "markdown", 500, 50, 0, "empty.md", "text/markdown");
        assert!(chunks.is_empty());

        let chunks2 = chunk_document_enriched("   \n\n  ", "auto", 500, 50, 0, "empty.txt", "text/plain");
        assert!(chunks2.is_empty());
    }

    #[test]
    fn test_chunk_indices_sequential() {
        let content = "# A\n\nText A content here.\n\n## B\n\nText B content here.\n\n## C\n\nText C content here.";
        let chunks = chunk_document_enriched(content, "markdown", 50, 10, 0, "doc.md", "text/markdown");

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i, "Chunk index should be sequential");
        }
    }

    #[test]
    fn test_offsets_are_valid() {
        let content = "# Title\n\nParagraph one.\n\n## Section\n\nParagraph two.";
        let chunks = chunk_document_enriched(content, "markdown", 500, 50, 0, "doc.md", "text/markdown");

        for chunk in &chunks {
            assert!(chunk.start_offset <= chunk.end_offset,
                "start_offset ({}) should be <= end_offset ({})", chunk.start_offset, chunk.end_offset);
        }
    }

    // ─── Backward compat tests ───

    #[test]
    fn test_legacy_chunk_document_still_works() {
        let text = "word ".repeat(100);
        let chunks = chunk_document(&text, "tokens", 50, 10);
        assert!(!chunks.is_empty());
        assert!(chunks[0].content_with_context.is_none(), "Legacy should not set content_with_context");
        assert!(chunks[0].heading_hierarchy.is_empty(), "Legacy should not set heading_hierarchy");
    }

    #[test]
    fn test_enriched_with_legacy_strategies() {
        let text = "# Title\n\nContent here.\n\n## Section\n\nMore content.";
        // Using "headings" strategy via enriched should still work (legacy path)
        let chunks = chunk_document_enriched(text, "headings", 1000, 0, 0, "doc.md", "text/markdown");
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("hi"), 1);
        assert!(estimate_tokens("hello world") > 0);
        // Russian text: should be conservative (more tokens per char due to multibyte)
        let russian = "Привет мир";
        assert!(estimate_tokens(russian) > 0);
    }

    #[test]
    fn test_html_to_markdown_like() {
        let html = "<h1>Title</h1><p>Hello</p><h2>Sub</h2><ul><li>Item 1</li><li>Item 2</li></ul>";
        let md = html_to_markdown_like(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("## Sub"));
        assert!(md.contains("Hello"));
        assert!(md.contains("- Item 1"));
        assert!(md.contains("- Item 2"));
    }

    #[test]
    fn test_build_context_prefix() {
        assert_eq!(build_context_prefix("file.md", &[]), "file.md");
        assert_eq!(
            build_context_prefix("file.md", &["Title".to_string(), "Section".to_string()]),
            "file.md > Title > Section"
        );
        assert_eq!(
            build_context_prefix("", &["Title".to_string()]),
            "Title"
        );
    }

    #[test]
    fn test_merge_small_chunks() {
        let chunks = vec![
            ChunkResult {
                content: "A".to_string(),
                content_with_context: Some("A".to_string()),
                heading_hierarchy: vec![],
                index: 0, start_offset: 0, end_offset: 1,
                metadata: serde_json::json!({}),
            },
            ChunkResult {
                content: "B".to_string(),
                content_with_context: Some("B".to_string()),
                heading_hierarchy: vec![],
                index: 1, start_offset: 2, end_offset: 3,
                metadata: serde_json::json!({}),
            },
        ];
        // Both chunks are tiny (1 token each), min is 10
        let merged = merge_small_chunks(chunks, 10);
        // First chunk stays (no previous to merge into), second merges into first
        assert_eq!(merged.len(), 1);
        assert!(merged[0].content.contains("A"));
        assert!(merged[0].content.contains("B"));
    }
}
