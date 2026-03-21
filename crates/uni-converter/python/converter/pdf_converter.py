"""PDF to Markdown converter using PyMuPDF (fitz)."""
import fitz  # pymupdf
from uni_bridge import progress


def convert_pdf(path: str, options: dict) -> dict:
    doc = fitz.open(path)
    pages = len(doc)
    progress(10, f"Opened PDF: {pages} pages")

    parts = []
    has_images = False
    has_tables = False

    for i, page in enumerate(doc):
        # Extract text with layout preservation
        text = page.get_text("text")
        if text.strip():
            parts.append(text.strip())

        # Check for images
        if page.get_images():
            has_images = True

        # Check for tables (heuristic: look for tab-aligned text)
        if "\t" in text or "│" in text:
            has_tables = True

        if pages > 10 and (i + 1) % 10 == 0:
            pct = 10 + int(80 * (i + 1) / pages)
            progress(pct, f"Processing page {i + 1}/{pages}...")

    doc.close()

    markdown = "\n\n".join(parts)

    # Try to extract title from first page
    title = None
    if parts:
        first_lines = parts[0].split("\n")
        for line in first_lines[:5]:
            stripped = line.strip()
            if stripped and len(stripped) < 200:
                title = stripped
                break

    progress(95, "Finalizing...")

    return {
        "markdown": markdown,
        "title": title,
        "metadata": {
            "original_format": "pdf",
            "pages": pages,
            "word_count": len(markdown.split()),
            "has_images": has_images,
            "has_tables": has_tables,
            "quality": "high",
            "python_converted": True,
        },
    }
