# @uni-fw/ui

React components, Mantine 8 theme, and settings modules for [UNI Framework](https://github.com/ts-vit/ai-chat) — reusable building blocks for AI desktop applications on Tauri 2.

## Features

- **UniProvider** — drop-in MantineProvider with UNI theme (dark mode, brand orange)
- **MarkdownRenderer** — react-markdown with syntax highlighting
- **Settings system** — `SettingsAdapter` interface, `TauriSettingsAdapter`, `useSettings()` hook
- **7 settings modules** — OpenRouter, Ollama, Generation, Interface, WebSearch, Budget, Terminal
- **Re-exports** — `@mantine/core`, `@mantine/hooks`, `@mantine/notifications`
- **ConfirmModal** — reusable confirmation dialog

## Installation
```bash
npm install @uni-fw/ui
```

### Peer Dependencies

- `react` >= 19
- `@mantine/core` >= 8
- `@mantine/hooks` >= 8
- `@mantine/notifications` >= 8
- `@tabler/icons-react`

## License

MIT
