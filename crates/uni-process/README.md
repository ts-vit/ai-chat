# uni-process

Child process management with async streaming for [UNI Framework](https://github.com/ts-vit/ai-chat).

## Features

- Async process spawning with `tokio::process`
- Streaming stdout/stderr via channels
- Stdin writing (bytes and lines)
- Process lifecycle management (status, kill, force_kill)
- Configurable kill timeout
- Environment variable injection
- Builder-pattern configuration
- No console window on Windows

## Usage

```rust
use uni_process::{ProcessConfig, ManagedProcess, ProcessEvent};

let config = ProcessConfig::new("echo")
    .arg("hello world")
    .cwd("/tmp")
    .env("MY_VAR", "value");

let mut process = ManagedProcess::spawn(config).await?;

while let Some(event) = process.next_event().await {
    match event {
        ProcessEvent::Stdout(data) => {
            print!("{}", String::from_utf8_lossy(&data));
        }
        ProcessEvent::Exited { code } => {
            println!("Process exited with code: {:?}", code);
            break;
        }
        _ => {}
    }
}
```

## License

MIT
