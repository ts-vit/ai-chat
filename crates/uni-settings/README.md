# uni-settings

File-based JSON settings store with atomic writes for [UNI Framework](https://github.com/ts-vit/ai-chat).

## Features

- `JsonSettingsStore` — read/write settings as JSON file
- Atomic writes via temporary file + rename
- `SettingsStore` trait for custom implementations
- Key-based get/set/delete/list operations
- Auto-detection and masking of sensitive values (API keys, passwords, tokens)
- Prefix-filtered key listing

## Usage
```toml
[dependencies]
uni-settings = "0.1"
```

## License

MIT
