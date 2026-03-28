# uni-http

HTTP client with retry, proxy support, and configurable timeouts for [UNI Framework](https://github.com/ts-vit/ai-chat).

## Features

- `build_http_client()` — configurable reqwest client with optional proxy (HTTP/SOCKS5)
- `retry_http_request()` — exponential backoff with Retry-After header support
- Timeout configuration
- Custom User-Agent

## Usage
```toml
[dependencies]
uni-http = "0.1"
```

## License

MIT
