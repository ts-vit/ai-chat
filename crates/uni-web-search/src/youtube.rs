use crate::content::fetch_content;
use crate::error::WebSearchError;
use crate::helpers::html_entities_decode;

/// Extract transcript from a YouTube video.
/// Tries native captions API first, falls back to Jina Reader.
pub async fn fetch_transcript(
    client: &reqwest::Client,
    url: &str,
) -> Result<String, WebSearchError> {
    let video_id = extract_video_id(url).ok_or_else(|| {
        WebSearchError::Parse(format!("Cannot extract video ID from URL: {}", url))
    })?;

    // Try to get captions from YouTube page
    match fetch_captions(client, &video_id).await {
        Ok(text) if !text.trim().is_empty() => return Ok(text),
        _ => {}
    }

    // Fallback: use Jina Reader
    let canonical = format!("https://www.youtube.com/watch?v={}", video_id);
    fetch_content(client, &canonical, 50000).await
}

/// Extract YouTube video ID from various URL formats
pub fn extract_video_id(url: &str) -> Option<String> {
    let re = regex::Regex::new(
        r"(?:v=|youtu\.be/|/(?:shorts|embed|v)/)([a-zA-Z0-9_-]{11})",
    )
    .unwrap();
    re.captures(url).map(|c| c[1].to_string())
}

async fn fetch_captions(
    client: &reqwest::Client,
    video_id: &str,
) -> Result<String, WebSearchError> {
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
        .map_err(|e| WebSearchError::Http(format!("YouTube fetch error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(WebSearchError::Api {
            provider: "YouTube".to_string(),
            status: resp.status().as_u16(),
            body: format!("HTTP {}", resp.status()),
        });
    }

    let html = resp
        .text()
        .await
        .map_err(|e| WebSearchError::Http(format!("YouTube read error: {}", e)))?;

    // Find captions track list in the page HTML
    let captions_start = html
        .find("\"captions\":")
        .or_else(|| html.find("\"captionTracks\":"))
        .ok_or_else(|| WebSearchError::Parse("No captions found in video".to_string()))?;

    // Extract captionTracks JSON array
    let tracks_start = html[captions_start..]
        .find("\"captionTracks\":")
        .ok_or_else(|| WebSearchError::Parse("No captionTracks found".to_string()))?;
    let start = captions_start + tracks_start + "\"captionTracks\":".len();

    // Find the array bounds
    let arr_start = html[start..]
        .find('[')
        .ok_or_else(|| WebSearchError::Parse("Cannot find captions array".to_string()))?;
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

    let end =
        end_pos.ok_or_else(|| WebSearchError::Parse("Cannot find end of captions array".to_string()))?;
    let tracks_json = &html[search_from..end];

    // Parse tracks
    let tracks: Vec<serde_json::Value> = serde_json::from_str(tracks_json)
        .map_err(|e| WebSearchError::Parse(format!("Cannot parse caption tracks: {}", e)))?;

    if tracks.is_empty() {
        return Err(WebSearchError::NoResults(
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
        .ok_or_else(|| WebSearchError::Parse("No caption track found".to_string()))?;

    let base_url = track
        .get("baseUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebSearchError::Parse("No baseUrl in caption track".to_string()))?;

    // Fetch caption XML
    let caption_resp = client
        .get(base_url)
        .send()
        .await
        .map_err(|e| WebSearchError::Http(format!("Caption fetch error: {}", e)))?;

    let caption_xml = caption_resp
        .text()
        .await
        .map_err(|e| WebSearchError::Http(format!("Caption read error: {}", e)))?;

    // Parse XML <text> tags and extract content
    let re_text = regex::Regex::new(r"<text[^>]*>([\s\S]*?)</text>").unwrap();
    let mut parts: Vec<String> = Vec::new();
    for cap in re_text.captures_iter(&caption_xml) {
        let text = html_entities_decode(&cap[1]);
        let cleaned = text.trim();
        if !cleaned.is_empty() {
            parts.push(cleaned.to_string());
        }
    }

    if parts.is_empty() {
        return Err(WebSearchError::NoResults(
            "No text in captions".to_string(),
        ));
    }

    Ok(parts.join(" "))
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
    fn test_extract_video_id_with_params() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=30s"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_extract_video_id_invalid() {
        assert!(extract_video_id("https://example.com").is_none());
        assert!(extract_video_id("not a url").is_none());
    }
}
