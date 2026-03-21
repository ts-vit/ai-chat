"""
UNI Bridge — Python-side SDK for UNI Framework scripts.

Provides JSON-RPC 2.0 communication with the Rust host process.
Scripts import this module and register handlers:

    from uni_bridge import App, progress

    app = App()

    @app.handler("convert")
    def convert(params):
        progress(50, "Processing...")
        return {"markdown": "# Hello", "title": "Doc"}

    app.run()
"""
import sys
import json
import logging
import traceback

# Configure logging to stderr (stdout is reserved for JSON-RPC)
logging.basicConfig(
    stream=sys.stderr,
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("uni_bridge")


def _send_json(data: dict):
    """Send a JSON message to stdout (Rust host)."""
    line = json.dumps(data, ensure_ascii=False)
    sys.stdout.write(line + "\n")
    sys.stdout.flush()


def progress(percent: int, message: str = ""):
    """Send a progress notification to the host."""
    _send_json({
        "jsonrpc": "2.0",
        "method": "progress",
        "params": {"percent": percent, "message": message},
    })


def _success_response(id: int, result):
    """Send a success response."""
    _send_json({"jsonrpc": "2.0", "id": id, "result": result})


def _error_response(id: int, code: int, message: str, data=None):
    """Send an error response."""
    err = {"code": code, "message": message}
    if data is not None:
        err["data"] = data
    _send_json({"jsonrpc": "2.0", "id": id, "error": err})


# Standard JSON-RPC error codes
PARSE_ERROR = -32700
INVALID_REQUEST = -32600
METHOD_NOT_FOUND = -32601
INVALID_PARAMS = -32602
INTERNAL_ERROR = -32603


class App:
    """Main application class for UNI Python scripts."""

    def __init__(self):
        self._handlers: dict[str, callable] = {}

    def handler(self, method: str):
        """Decorator to register a method handler."""
        def decorator(func):
            self._handlers[method] = func
            return func
        return decorator

    def run(self):
        """
        Main loop: read one JSON-RPC request from stdin, dispatch, respond.
        For one-shot mode: process one request and exit.
        """
        logger.debug("UNI Bridge started, waiting for request...")

        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue

            try:
                request = json.loads(line)
            except json.JSONDecodeError as e:
                _error_response(0, PARSE_ERROR, f"JSON parse error: {e}")
                continue

            # Validate JSON-RPC
            if request.get("jsonrpc") != "2.0":
                _error_response(
                    request.get("id", 0),
                    INVALID_REQUEST,
                    "Missing or invalid jsonrpc version",
                )
                continue

            req_id = request.get("id", 0)
            method = request.get("method", "")
            params = request.get("params", {})

            logger.debug(f"Received request: method={method}, id={req_id}")

            if method not in self._handlers:
                _error_response(req_id, METHOD_NOT_FOUND, f"Unknown method: {method}")
                continue

            try:
                result = self._handlers[method](params)
                _success_response(req_id, result)
            except Exception as e:
                logger.error(f"Handler error: {traceback.format_exc()}")
                _error_response(req_id, INTERNAL_ERROR, str(e))

        logger.debug("UNI Bridge finished (stdin closed)")
