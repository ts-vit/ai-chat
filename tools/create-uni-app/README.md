# @uni-fw/create-app

Scaffold a new [UNI Framework](https://github.com/ts-vit/ai-chat) application — Tauri 2 + React 19 + Rust.

## Usage

### Standalone project (separate repository)

```bash
npx @uni-fw/create-app --name my-app --output /path/to/my-app --modules ssh-tunnel,terminal
```

### Inside monorepo

```bash
npx @uni-fw/create-app --name my-app --modules ssh-tunnel
```

### Interactive mode

```bash
npx @uni-fw/create-app
```

## Available modules

**Settings UI:** openrouter, ollama, generation, interface, web-search, budget

**Platform (UI + Rust):** ssh-tunnel, terminal

**Rust only:** uni-http, uni-llm, uni-embedding, uni-search, uni-audio, uni-python, uni-converter

## License

MIT
