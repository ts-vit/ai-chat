use regex::Regex;

use crate::types::*;

/// Convert YouTube video to Markdown (via captions/subtitles).
///
/// Strategy:
/// 1. Extract video ID from URL
/// 2. Fetch YouTube page, parse caption tracks
/// 3. Fetch caption XML, extract text
/// 4. Fallback: Jina Reader
pub async fn convert_youtube(
    client: &reqwest::Client,
    url: &str,
) -> Result<ConversionResult, uni_common::UniError> {
    let video_id = extract_video_id(url).ok_or_else(|| {
        uni_common::UniError::Generic(format!("Cannot extract video ID from URL: {}", url))
    })?;

    // Try direct captions
    let transcript = match fetch_captions(client, &video_id).await {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) => {
            log::debug!(
                "YouTube captions empty for {}, trying Jina Reader",
                video_id
            );
            jina_youtube_fallback(client, &video_id).await?
        }
        Err(e) => {
            log::debug!(
                "YouTube captions failed for {}: {}, trying Jina Reader",
                video_id,
                e
            );
            jina_youtube_fallback(client, &video_id).await?
        }
    };

    if transcript.trim().is_empty() {
        return Err(uni_common::UniError::Generic(
            "No transcript extracted from YouTube video".to_string(),
        ));
    }

    // Format as Markdown
    let markdown = format!(
        "# YouTube Transcript\n\nSource: https://www.youtube.com/watch?v={}\n\n{}",
        video_id,
        transcript.trim()
    );

    Ok(ConversionResult {
        markdown: markdown.clone(),
        title: Some(format!("YouTube: {}", video_id)),
        metadata: ConversionMeta {
            original_format: "youtube".to_string(),
            pages: None,
            word_count: count_words(&markdown),
            has_images: false,
            has_tables: false,
            quality: ConversionQuality::Medium,
            python_converted: false,
        },
    })
}

async fn jina_youtube_fallback(
    client: &reqwest::Client,
    video_id: &str,
) -> Result<String, uni_common::UniError> {
    let canonical = format!("https://www.youtube.com/watch?v={}", video_id);
    let jina_url = format!("https://r.jina.ai/{}", canonical);
    let resp = client
        .get(&jina_url)
        .header("Accept", "text/markdown")
        .send()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("Jina fallback error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(uni_common::UniError::Generic(
            "Failed to fetch YouTube transcript via both captions and Jina Reader".to_string(),
        ));
    }

    resp.text()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("Jina read error: {}", e)))
}

/// Extract YouTube video ID from various URL formats.
pub fn extract_video_id(url: &str) -> Option<String> {
    let re = Regex::new(r"(?:v=|youtu\.be/|/(?:shorts|embed|v)/)([a-zA-Z0-9_-]{11})").unwrap();
    re.captures(url).map(|c| c[1].to_string())
}

/// Fetch captions directly from YouTube page.
async fn fetch_captions(
    client: &reqwest::Client,
    video_id: &str,
) -> Result<String, uni_common::UniError> {
    let page_url = format!("https://www.youtube.com/watch?v={}", video_id);
    let resp = client
        .get(&page_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("YouTube fetch error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(uni_common::UniError::Generic(format!(
            "YouTube returned {}",
            resp.status()
        )));
    }

    let html = resp
        .text()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("YouTube read error: {}", e)))?;

    // Find captionTracks in page HTML
    let captions_start = html
        .find("\"captions\":")
        .or_else(|| html.find("\"captionTracks\":"))
        .ok_or_else(|| uni_common::UniError::Generic("No captions found in video".to_string()))?;

    let tracks_start = html[captions_start..]
        .find("\"captionTracks\":")
        .ok_or_else(|| uni_common::UniError::Generic("No captionTracks found".to_string()))?;
    let start = captions_start + tracks_start + "\"captionTracks\":".len();

    let arr_start = html[start..]
        .find('[')
        .ok_or_else(|| uni_common::UniError::Generic("Cannot find captions array".to_string()))?;
    let search_from = start + arr_start;

    // Find matching closing bracket
    let mut depth = 0;
    let mut end_pos = None;
    for (i, ch) in html[search_from..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end_pos = Some(search_from + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    let end = end_pos.ok_or_else(|| {
        uni_common::UniError::Generic("Cannot find end of captions array".to_string())
    })?;
    let tracks_json = &html[search_from..end];

    let tracks: Vec<serde_json::Value> = serde_json::from_str(tracks_json).map_err(|e| {
        uni_common::UniError::Generic(format!("Cannot parse caption tracks: {}", e))
    })?;

    if tracks.is_empty() {
        return Err(uni_common::UniError::Generic(
            "No caption tracks available".to_string(),
        ));
    }

    // Prefer English, then first available
    let track = tracks
        .iter()
        .find(|t| {
            t.get("languageCode")
                .and_then(|v| v.as_str())
                .map(|l| l.starts_with("en"))
                .unwrap_or(false)
        })
        .or(tracks.first())
        .ok_or_else(|| uni_common::UniError::Generic("No caption track found".to_string()))?;

    let base_url = track
        .get("baseUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| uni_common::UniError::Generic("No baseUrl in caption track".to_string()))?;

    // Fetch caption XML
    let caption_resp = client
        .get(base_url)
        .send()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("Caption fetch error: {}", e)))?;

    let caption_xml = caption_resp
        .text()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("Caption read error: {}", e)))?;

    // Parse <text> tags
    let re_text = Regex::new(r"<text[^>]*>([\s\S]*?)</text>").unwrap();
    let mut parts: Vec<String> = Vec::new();
    for cap in re_text.captures_iter(&caption_xml) {
        let text = html_entities_decode(&cap[1]);
        let cleaned = text.trim();
        if !cleaned.is_empty() {
            parts.push(cleaned.to_string());
        }
    }

    if parts.is_empty() {
        return Err(uni_common::UniError::Generic(
            "No text in captions".to_string(),
        ));
    }

    Ok(parts.join(" "))
}

fn html_entities_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("\\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id_watch() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_extract_video_id_short_url() {
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_extract_video_id_shorts() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/shorts/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_extract_video_id_embed() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/embed/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_extract_video_id_invalid() {
        assert_eq!(extract_video_id("https://example.com"), None);
        assert_eq!(extract_video_id("not a url"), None);
    }

    #[test]
    fn test_html_entities_decode() {
        assert_eq!(html_entities_decode("hello &amp; world"), "hello & world");
        assert_eq!(html_entities_decode("&lt;b&gt;"), "<b>");
        assert_eq!(html_entities_decode("it&#39;s"), "it's");
    }
}
