// RAG Context Builder: constructs context and system instructions from KB search results

use crate::services::kb_search::KbSearchResultItem;

const DEFAULT_RAG_INSTRUCTION: &str = "\
Answer the user's question based on the provided knowledge base context. \
Cite your sources by referencing the document name when possible. \
If the context doesn't contain relevant information, say so honestly. \
Do not make up information that isn't in the provided context.";

/// Build XML context block from search results for injection into LLM messages.
pub fn build_rag_context(results: &[KbSearchResultItem], kb_name: &str) -> String {
    if results.is_empty() {
        return String::new();
    }

    let mut ctx = format!("<knowledge_base name=\"{}\">\n", kb_name);
    for r in results {
        ctx.push_str(&format!(
            "<source document=\"{}\" chunk=\"{}\" relevance=\"{:.2}\">\n{}\n</source>\n",
            r.document_name, r.chunk_index, r.score, r.content
        ));
    }
    ctx.push_str("</knowledge_base>");
    ctx
}

/// Build RAG system instruction to prepend to the system prompt.
/// Uses the KB's custom system_prompt if set, otherwise the default.
pub fn build_rag_system_instruction(kb_system_prompt: &str) -> String {
    if kb_system_prompt.trim().is_empty() {
        DEFAULT_RAG_INSTRUCTION.to_string()
    } else {
        kb_system_prompt.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(doc: &str, chunk_idx: i64, score: f32, content: &str) -> KbSearchResultItem {
        KbSearchResultItem {
            chunk_id: format!("chunk_{}", chunk_idx),
            document_id: format!("doc_{}", doc),
            document_name: doc.to_string(),
            content: content.to_string(),
            chunk_index: chunk_idx,
            score,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn test_empty_results_empty_string() {
        assert_eq!(build_rag_context(&[], "test_kb"), "");
    }

    #[test]
    fn test_single_result_xml_format() {
        let items = vec![make_item("readme.md", 0, 0.95, "Hello world")];
        let ctx = build_rag_context(&items, "my_kb");
        assert!(ctx.starts_with("<knowledge_base name=\"my_kb\">"));
        assert!(ctx.ends_with("</knowledge_base>"));
        assert!(ctx.contains("document=\"readme.md\""));
        assert!(ctx.contains("chunk=\"0\""));
        assert!(ctx.contains("relevance=\"0.95\""));
        assert!(ctx.contains("Hello world"));
    }

    #[test]
    fn test_multiple_results() {
        let items = vec![
            make_item("doc1.md", 0, 0.9, "Content 1"),
            make_item("doc2.md", 1, 0.8, "Content 2"),
            make_item("doc3.md", 2, 0.7, "Content 3"),
        ];
        let ctx = build_rag_context(&items, "kb");
        let source_count = ctx.matches("<source ").count();
        assert_eq!(source_count, 3);
    }

    #[test]
    fn test_special_chars_in_content() {
        let items = vec![make_item("doc.md", 0, 0.9, "x < y && a > b & \"quoted\"")];
        let ctx = build_rag_context(&items, "kb");
        // Currently no XML escaping — test documents existing behavior
        assert!(ctx.contains("x < y && a > b & \"quoted\""));
    }

    #[test]
    fn test_russian_content() {
        let items = vec![make_item("docs.md", 0, 0.85, "Привет мир, это русский текст")];
        let ctx = build_rag_context(&items, "kb");
        assert!(ctx.contains("Привет мир"));
    }

    #[test]
    fn test_build_rag_instruction_default() {
        let result = build_rag_system_instruction("");
        assert!(result.contains("Answer the user's question"));
    }

    #[test]
    fn test_build_rag_instruction_whitespace_only() {
        let result = build_rag_system_instruction("   \n  ");
        assert!(result.contains("Answer the user's question"));
    }

    #[test]
    fn test_build_rag_instruction_custom() {
        let custom = "Use the following context to answer.";
        assert_eq!(build_rag_system_instruction(custom), custom);
    }
}
