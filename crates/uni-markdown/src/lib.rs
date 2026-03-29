//! uni-markdown — Markdown parsing and transformation for UNI Framework
//!
//! Powered by pulldown-cmark. Provides AST-based parsing, section splitting,
//! content extraction, transformations, and rendering helpers.

pub mod extract;
pub mod parse;
pub mod render;
pub mod sections;
pub mod transform;

// Re-export commonly used types
pub use extract::{
    reading_time_minutes, table_of_contents, table_of_contents_md, to_plain_text, word_count,
    TocEntry,
};
pub use parse::{extract_code_blocks, extract_headings, extract_links, CodeBlock, Heading, Link};
pub use render::{render_code_block, render_heading, render_link, render_list, render_table};
pub use sections::{
    find_section, hierarchy_at_offset, split_all_sections, split_sections, Section,
};
pub use transform::{insert_section_after, remove_section, replace_section, strip_formatting};
