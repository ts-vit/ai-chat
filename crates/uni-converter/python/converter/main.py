"""
UNI Document Converter — converts documents to Markdown.

Supported formats: PDF, DOCX, XLSX, PPTX, EPUB.
Uses UNI Bridge for JSON-RPC communication with Rust host.
"""
import sys
import os

# Add the lib directory to path so we can import uni_bridge
lib_dir = os.environ.get("UNI_LIB_DIR", "")
if lib_dir and lib_dir not in sys.path:
    sys.path.insert(0, lib_dir)

from uni_bridge import App, progress

app = App()


@app.handler("convert")
def convert(params: dict) -> dict:
    """
    Convert a document to Markdown.

    Params:
        input_path: str — path to the input file
        format: str — file format (pdf, docx, xlsx, pptx, epub)
        options: dict — optional conversion settings

    Returns:
        markdown: str — converted content
        title: str|None — extracted title
        metadata: dict — conversion metadata
    """
    input_path = params.get("input_path")
    fmt = params.get("format", "").lower()
    options = params.get("options", {})

    if not input_path or not os.path.exists(input_path):
        raise ValueError(f"Input file not found: {input_path}")

    progress(5, f"Starting {fmt.upper()} conversion...")

    converters = {
        "pdf": _convert_pdf,
        "docx": _convert_docx,
        "xlsx": _convert_xlsx,
        "pptx": _convert_pptx,
        "epub": _convert_epub,
    }

    converter = converters.get(fmt)
    if not converter:
        raise ValueError(f"Unsupported format: {fmt}")

    result = converter(input_path, options)
    progress(100, "Conversion complete")
    return result


def _convert_pdf(path: str, options: dict) -> dict:
    from pdf_converter import convert_pdf
    return convert_pdf(path, options)


def _convert_docx(path: str, options: dict) -> dict:
    from docx_converter import convert_docx
    return convert_docx(path, options)


def _convert_xlsx(path: str, options: dict) -> dict:
    from xlsx_converter import convert_xlsx
    return convert_xlsx(path, options)


def _convert_pptx(path: str, options: dict) -> dict:
    from pptx_converter import convert_pptx
    return convert_pptx(path, options)


def _convert_epub(path: str, options: dict) -> dict:
    from epub_converter import convert_epub
    return convert_epub(path, options)


app.run()
