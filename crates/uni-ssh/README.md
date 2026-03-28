# uni-ssh

SSH tunnel with local SOCKS5 proxy and auto-reconnect for [UNI Framework](https://github.com/ts-vit/ai-chat).

## Features

- SSH tunnel establishment (password and key auth)
- Local SOCKS5 proxy listener
- Auto-reconnect on connection loss
- Proxy URL resolution for HTTP clients
- Built on `russh`

## Usage
```toml
[dependencies]
uni-ssh = "0.1"
```

## License

MIT
