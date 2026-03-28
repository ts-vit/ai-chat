# uni-common

Common types, utilities, and error handling for [UNI Framework](https://github.com/ts-vit/ai-chat) — a set of reusable Rust crates and React components for building AI desktop applications with Tauri 2.

## Features

- `UniError` — unified error type across all UNI crates
- `generate_id()` — UUID v4 generation
- `now_unix_secs()` — Unix timestamp in seconds
- `safe_truncate()` / `safe_truncate_chars()` — UTF-8 safe string truncation
- `estimate_tokens()` — approximate token count
- Re-export of `tokio_util::sync::CancellationToken`

## Usage
```toml
[dependencies]
uni-common = "0.1"
```

## License

MIT
