"""EPUB to Markdown converter using ebooklib + basic HTML stripping."""
import re
import ebooklib
from ebooklib import epub
from uni_bridge import progress


def convert_epub(path: str, options: dict) -> dict:
    progress(10, "Reading EPUB...")

    book = epub.read_epub(path, options={"ignore_ncx": True})

    title_meta = book.get_metadata("DC", "title")
    title = title_meta[0][0] if title_meta else None

    progress(20, "Extracting chapters...")

    items = list(book.get_items_of_type(ebooklib.ITEM_DOCUMENT))
    parts = []
    has_images = False

    for idx, item in enumerate(items):
        content = item.get_content().decode("utf-8", errors="replace")

        text = _html_to_text(content)
        if text.strip():
            parts.append(text.strip())

        if "<img" in content.lower():
            has_images = True

        if len(items) > 5 and (idx + 1) % 5 == 0:
            pct = 20 + int(70 * (idx + 1) / len(items))
            progress(pct, f"Processing chapter {idx + 1}/{len(items)}...")

    markdown = "\n\n---\n\n".join(parts)

    progress(95, "Finalizing...")

    return {
        "markdown": markdown,
        "title": title,
        "metadata": {
            "original_format": "epub",
            "pages": len(items),
            "word_count": len(markdown.split()),
            "has_images": has_images,
            "has_tables": False,
            "quality": "medium",
            "python_converted": True,
        },
    }


def _html_to_text(html: str) -> str:
    """Basic HTML to Markdown-like text conversion."""
    html = re.sub(r"<h1[^>]*>(.*?)</h1>", r"# \1\n", html, flags=re.DOTALL)
    html = re.sub(r"<h2[^>]*>(.*?)</h2>", r"## \1\n", html, flags=re.DOTALL)
    html = re.sub(r"<h3[^>]*>(.*?)</h3>", r"### \1\n", html, flags=re.DOTALL)
    html = re.sub(r"<h[4-6][^>]*>(.*?)</h[4-6]>", r"#### \1\n", html, flags=re.DOTALL)

    html = re.sub(r"<br\s*/?>", "\n", html)
    html = re.sub(r"<p[^>]*>", "\n\n", html)
    html = re.sub(r"</p>", "", html)

    html = re.sub(
        r"<(?:b|strong)[^>]*>(.*?)</(?:b|strong)>",
        r"**\1**",
        html,
        flags=re.DOTALL,
    )
    html = re.sub(
        r"<(?:i|em)[^>]*>(.*?)</(?:i|em)>",
        r"*\1*",
        html,
        flags=re.DOTALL,
    )

    html = re.sub(
        r'<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>',
        r"[\2](\1)",
        html,
        flags=re.DOTALL,
    )

    html = re.sub(r"<li[^>]*>", "- ", html)
    html = re.sub(r"</li>", "\n", html)

    html = re.sub(r"<script[^>]*>.*?</script>", "", html, flags=re.DOTALL)
    html = re.sub(r"<style[^>]*>.*?</style>", "", html, flags=re.DOTALL)
    html = re.sub(r"<[^>]+>", "", html)

    html = html.replace("&amp;", "&")
    html = html.replace("&lt;", "<")
    html = html.replace("&gt;", ">")
    html = html.replace("&quot;", '"')
    html = html.replace("&#39;", "'")
    html = html.replace("&nbsp;", " ")

    html = re.sub(r"[ \t]+", " ", html)
    html = re.sub(r"\n{3,}", "\n\n", html)

    return html.strip()
