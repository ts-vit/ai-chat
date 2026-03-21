use crate::types::*;

/// Convert CSV content to a Markdown table.
pub fn convert_csv(
    content: &str,
    file_name: Option<&str>,
) -> Result<ConversionResult, uni_common::UniError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| uni_common::UniError::Generic(format!("CSV header error: {}", e)))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    if headers.is_empty() {
        return Err(uni_common::UniError::Generic(
            "CSV has no columns".to_string(),
        ));
    }

    let mut md = String::new();

    // Header row
    md.push_str("| ");
    md.push_str(&headers.join(" | "));
    md.push_str(" |\n");

    // Separator row
    md.push_str("| ");
    md.push_str(
        &headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | "),
    );
    md.push_str(" |\n");

    // Data rows
    for record in reader.records() {
        let record =
            record.map_err(|e| uni_common::UniError::Generic(format!("CSV row error: {}", e)))?;
        md.push_str("| ");
        let cells: Vec<&str> = record.iter().collect();
        // Pad to header length if needed
        let mut row_cells: Vec<String> = cells.iter().map(|c| c.to_string()).collect();
        while row_cells.len() < headers.len() {
            row_cells.push(String::new());
        }
        md.push_str(&row_cells[..headers.len()].join(" | "));
        md.push_str(" |\n");
    }

    let title = file_name.map(|n| {
        std::path::Path::new(n)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(n)
            .to_string()
    });

    let word_count = count_words(&md);

    Ok(ConversionResult {
        markdown: md,
        title,
        metadata: ConversionMeta {
            original_format: "csv".to_string(),
            pages: None,
            word_count,
            has_images: false,
            has_tables: true,
            quality: ConversionQuality::High,
            python_converted: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_csv_basic() {
        let csv = "Name,Age,City\nAlice,30,NYC\nBob,25,LA";
        let result = convert_csv(csv, Some("people.csv")).unwrap();
        assert!(result.markdown.contains("| Name | Age | City |"));
        assert!(result.markdown.contains("| --- | --- | --- |"));
        assert!(result.markdown.contains("| Alice | 30 | NYC |"));
        assert!(result.markdown.contains("| Bob | 25 | LA |"));
        assert!(result.metadata.has_tables);
        assert_eq!(result.metadata.quality, ConversionQuality::High);
        assert_eq!(result.title.as_deref(), Some("people"));
    }

    #[test]
    fn test_convert_csv_single_column() {
        let csv = "Item\napple\nbanana";
        let result = convert_csv(csv, None).unwrap();
        assert!(result.markdown.contains("| Item |"));
        assert!(result.markdown.contains("| apple |"));
    }

    #[test]
    fn test_convert_csv_with_commas_in_values() {
        let csv = "Name,Description\nAlice,\"Hello, world\"\nBob,\"Line1\nLine2\"";
        let result = convert_csv(csv, None).unwrap();
        assert!(result.markdown.contains("Hello, world"));
    }

    #[test]
    fn test_convert_csv_russian() {
        let csv = "Имя,Возраст\nАлиса,30\nБорис,25";
        let result = convert_csv(csv, None).unwrap();
        assert!(result.markdown.contains("| Имя | Возраст |"));
        assert!(result.markdown.contains("| Алиса | 30 |"));
    }

    #[test]
    fn test_convert_csv_empty() {
        let csv = "";
        let result = convert_csv(csv, None);
        // Empty CSV should error (no columns)
        assert!(result.is_err());
    }
}
