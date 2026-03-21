"""DOCX to Markdown converter using mammoth."""
import logging
import mammoth
from uni_bridge import progress


def convert_docx(path: str, options: dict) -> dict:
    progress(10, "Reading DOCX...")

    with open(path, "rb") as f:
        result = mammoth.convert_to_markdown(f)

    markdown = result.value
    messages = result.messages

    if messages:
        log = logging.getLogger("docx_converter")
        for msg in messages[:5]:
            log.warning("mammoth: %s", msg)

    progress(80, "Extracting metadata...")

    title = None
    for line in markdown.split("\n"):
        stripped = line.strip()
        if stripped.startswith("# "):
            title = stripped[2:].strip()
            break

    has_tables = "|" in markdown and "---" in markdown
    has_images = "![" in markdown

    progress(95, "Finalizing...")

    return {
        "markdown": markdown,
        "title": title,
        "metadata": {
            "original_format": "docx",
            "pages": None,
            "word_count": len(markdown.split()),
            "has_images": has_images,
            "has_tables": has_tables,
            "quality": "medium",
            "python_converted": True,
        },
    }
