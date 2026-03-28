# @uni-fw/terminal-ui

Terminal panel UI component with xterm.js for [UNI Framework](https://github.com/ts-vit/ai-chat).

## Features

- `TerminalPanel` — tabbed terminal interface with PTY support
- Multiple terminal sessions with tab management
- xterm.js integration with fit addon
- PTY create/write/resize/kill via Tauri commands
- Event-based output streaming (pty-data, pty-exit)

## Installation
```bash
npm install @uni-fw/terminal-ui
```

### Peer Dependencies

- `react` >= 19
- `@mantine/core` >= 8
- `@xterm/xterm`
- `@xterm/addon-fit`
- `@tauri-apps/api` >= 2

## License

MIT
