/// Fetches readable content from a URL via Jina Reader or raw HTML fallback.
pub async fn fetch_content(client: &reqwest::Client, url: &str, max_words: usize) -> Result<String, String> {
    match fetch_via_jina(client, url, max_words).await {
        Ok(content) if !content.trim().is_empty() => Ok(content),
        _ => fetch_raw_fallback(client, url, max_words).await,
    }
}

async fn fetch_via_jina(client: &reqwest::Client, url: &str, max_words: usize) -> Result<String, String> {
    let jina_url = format!("https://r.jina.ai/{}", url);
    let resp = client
        .get(&jina_url)
        .header("Accept", "text/markdown")
        .send()
        .await
        .map_err(|e| format!("Jina Reader error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Jina Reader returned {}", resp.status()));
    }

    let text = resp
        .text()
        .await
        .map_err(|e| format!("Jina Reader read error: {}", e))?;

    Ok(truncate_by_words(&text, max_words))
}

async fn fetch_raw_fallback(client: &reqwest::Client, url: &str, max_words: usize) -> Result<String, String> {
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (compatible; AIChat/1.0)")
        .send()
        .await
        .map_err(|e| format!("Fetch error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let html = resp
        .text()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    let text = strip_html_tags(&html);
    Ok(truncate_by_words(&text, max_words))
}

pub fn strip_html_tags(html: &str) -> String {
    let re_tags = regex::Regex::new(r"<(script|style|noscript)[^>]*>[\s\S]*?</\1>").unwrap();
    let cleaned = re_tags.replace_all(html, " ");
    let re_tag = regex::Regex::new(r"<[^>]+>").unwrap();
    let text = re_tag.replace_all(&cleaned, " ");
    let re_ws = regex::Regex::new(r"\s+").unwrap();
    re_ws.replace_all(&text, " ").trim().to_string()
}

fn truncate_by_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.to_string();
    }
    words[..max_words].join(" ")
}

/// Extract YouTube video transcript via captions API or Jina Reader fallback.
pub async fn fetch_youtube_transcript(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let video_id = extract_youtube_video_id(url)
        .ok_or_else(|| format!("Cannot extract video ID from URL: {}", url))?;

    // Try to get captions from YouTube page
    match fetch_youtube_captions(client, &video_id).await {
        Ok(text) if !text.trim().is_empty() => return Ok(text),
        _ => {}
    }

    // Fallback: use Jina Reader
    let canonical = format!("https://www.youtube.com/watch?v={}", video_id);
    fetch_content(client, &canonical, 50000).await
}

fn extract_youtube_video_id(url: &str) -> Option<String> {
    // Try regex patterns for all YouTube URL formats
    let re = regex::Regex::new(
        r"(?:v=|youtu\.be/|/(?:shorts|embed|v)/)([a-zA-Z0-9_-]{11})"
    ).unwrap();
    re.captures(url).map(|c| c[1].to_string())
}

async fn fetch_youtube_captions(client: &reqwest::Client, video_id: &str) -> Result<String, String> {
    let page_url = format!("https://www.youtube.com/watch?v={}", video_id);
    let resp = client
        .get(&page_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| format!("YouTube fetch error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("YouTube returned {}", resp.status()));
    }

    let html = resp.text().await.map_err(|e| format!("YouTube read error: {}", e))?;

    // Find captions track list in the page HTML
    let captions_start = html.find("\"captions\":")
        .or_else(|| html.find("\"captionTracks\":"))
        .ok_or("No captions found in video")?;

    // Extract captionTracks JSON array
    let tracks_start = html[captions_start..].find("\"captionTracks\":")
        .ok_or("No captionTracks found")?;
    let start = captions_start + tracks_start + "\"captionTracks\":".len();

    // Find the array bounds
    let arr_start = html[start..].find('[')
        .ok_or("Cannot find captions array")?;
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

    let end = end_pos.ok_or("Cannot find end of captions array")?;
    let tracks_json = &html[search_from..end];

    // Parse tracks
    let tracks: Vec<serde_json::Value> = serde_json::from_str(tracks_json)
        .map_err(|e| format!("Cannot parse caption tracks: {}", e))?;

    if tracks.is_empty() {
        return Err("No caption tracks available".to_string());
    }

    // Prefer English, then first available
    let track = tracks.iter()
        .find(|t| {
            t.get("languageCode")
                .and_then(|v| v.as_str())
                .map(|l| l.starts_with("en"))
                .unwrap_or(false)
        })
        .or(tracks.first())
        .ok_or("No caption track found")?;

    let base_url = track.get("baseUrl")
        .and_then(|v| v.as_str())
        .ok_or("No baseUrl in caption track")?;

    // Fetch caption XML
    let caption_resp = client
        .get(base_url)
        .send()
        .await
        .map_err(|e| format!("Caption fetch error: {}", e))?;

    let caption_xml = caption_resp.text().await
        .map_err(|e| format!("Caption read error: {}", e))?;

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
        return Err("No text in captions".to_string());
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
    fn test_truncate_by_words_short() {
        let text = "one two three";
        assert_eq!(truncate_by_words(text, 10), text);
    }

    #[test]
    fn test_truncate_by_words_long() {
        let text = "one two three four five six seven";
        let result = truncate_by_words(text, 3);
        assert_eq!(result, "one two three");
    }

    #[test]
    fn test_truncate_by_words_russian() {
        let text = "Привет мир это тест юникода";
        let result = truncate_by_words(text, 2);
        assert_eq!(result, "Привет мир");
    }

    #[test]
    fn test_truncate_by_words_exact_limit() {
        let text = "one two three";
        assert_eq!(truncate_by_words(text, 3), text);
    }

    #[test]
    fn test_truncate_by_words_empty() {
        assert_eq!(truncate_by_words("", 10), "");
    }

    #[test]
    fn test_extract_youtube_video_id_watch() {
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_extract_youtube_video_id_short_url() {
        assert_eq!(
            extract_youtube_video_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_extract_youtube_video_id_shorts() {
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/shorts/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_extract_youtube_video_id_embed() {
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/embed/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_html_entities_decode() {
        assert_eq!(html_entities_decode("hello &amp; world"), "hello & world");
        assert_eq!(html_entities_decode("&lt;b&gt;bold&lt;/b&gt;"), "<b>bold</b>");
        assert_eq!(html_entities_decode("it&#39;s"), "it's");
    }
}
