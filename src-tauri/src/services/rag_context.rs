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
