use crate::sections::find_section;

/// Replace a section's content by heading text.
/// The heading line itself is preserved, only the body is replaced.
/// Returns the full document with the section replaced.
pub fn replace_section(markdown: &str, heading_text: &str, new_body: &str) -> Option<String> {
    let section = find_section(markdown, heading_text)?;

    let mut result = String::new();
    result.push_str(&markdown[..section.start_offset]);

    if let Some(heading) = &section.heading {
        let heading_line = &markdown[heading.start_offset..heading.end_offset];
        result.push_str(heading_line);
        result.push_str("\n\n");
        result.push_str(new_body.trim());
        result.push_str("\n\n");
    } else {
        result.push_str(new_body.trim());
        result.push_str("\n\n");
    }

    result.push_str(&markdown[section.end_offset..]);

    Some(result)
}

/// Insert a new section after a specified heading.
/// If `after_heading` is None, insert at the end.
pub fn insert_section_after(
    markdown: &str,
    after_heading: Option<&str>,
    new_heading_level: u8,
    new_heading_text: &str,
    new_body: &str,
) -> String {
    let hashes = "#".repeat(new_heading_level as usize);
    let new_section = format!(
        "\n\n{} {}\n\n{}\n",
        hashes,
        new_heading_text,
        new_body.trim()
    );

    match after_heading {
        Some(heading_text) => {
            if let Some(section) = find_section(markdown, heading_text) {
                let mut result = String::new();
                result.push_str(&markdown[..section.end_offset]);
                result.push_str(&new_section);
                result.push_str(&markdown[section.end_offset..]);
                result
            } else {
                format!("{}{}", markdown, new_section)
            }
        }
        None => {
            format!("{}{}", markdown, new_section)
        }
    }
}

/// Remove a section by heading text (including heading and body).
pub fn remove_section(markdown: &str, heading_text: &str) -> Option<String> {
    let section = find_section(markdown, heading_text)?;

    let mut result = String::new();
    result.push_str(&markdown[..section.start_offset]);
    result.push_str(&markdown[section.end_offset..]);

    let cleaned = result
        .replace("\n\n\n\n", "\n\n")
        .replace("\n\n\n", "\n\n");
    Some(cleaned)
}

/// Strip markdown formatting, returning plain text.
pub fn strip_formatting(markdown: &str) -> String {
    crate::extract::to_plain_text(markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_section() {
        let md = "# Title\n\nIntro\n\n## Auth\n\nOld content\n\n## Settings\n\nConfig";
        let result = replace_section(md, "Auth", "New auth content").unwrap();
        assert!(result.contains("New auth content"));
        assert!(!result.contains("Old content"));
        assert!(result.contains("## Auth"));
        assert!(result.contains("## Settings"));
        assert!(result.contains("Config"));
    }

    #[test]
    fn test_replace_section_not_found() {
        let md = "# Title\n\nContent";
        assert!(replace_section(md, "Nonexistent", "new").is_none());
    }

    #[test]
    fn test_insert_section_after() {
        let md = "# Title\n\nIntro\n\n## Auth\n\nAuth content";
        let result = insert_section_after(md, Some("Auth"), 2, "Settings", "Config values");
        assert!(result.contains("## Settings"));
        assert!(result.contains("Config values"));
        let auth_pos = result.find("## Auth").unwrap();
        let settings_pos = result.find("## Settings").unwrap();
        assert!(settings_pos > auth_pos);
    }

    #[test]
    fn test_insert_section_at_end() {
        let md = "# Title\n\nContent";
        let result = insert_section_after(md, None, 2, "New", "Body");
        assert!(result.contains("## New"));
        assert!(result.contains("Body"));
    }

    #[test]
    fn test_remove_section() {
        let md = "# Title\n\nIntro\n\n## Auth\n\nAuth content\n\n## Settings\n\nConfig";
        let result = remove_section(md, "Auth").unwrap();
        assert!(!result.contains("## Auth"));
        assert!(!result.contains("Auth content"));
        assert!(result.contains("# Title"));
        assert!(result.contains("## Settings"));
    }

    #[test]
    fn test_remove_section_not_found() {
        let md = "# Title\n\nContent";
        assert!(remove_section(md, "Nonexistent").is_none());
    }

    #[test]
    fn test_strip_formatting() {
        let md = "**Bold** and `code`";
        let plain = strip_formatting(md);
        assert!(plain.contains("Bold"));
        assert!(plain.contains("code"));
        assert!(!plain.contains("**"));
        assert!(!plain.contains("`"));
    }
}
