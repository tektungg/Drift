# Drift

A desktop application built with Tauri 2 and Rust.

## Prerequisites

- [Rust](https://rustup.rs/) 1.70+
- [Tauri CLI](https://tauri.app/start/prerequisites/) v2
- Node.js (optional, for future frontend tooling)

## Development

```bash
# Install Tauri CLI (first time only)
cargo install tauri-cli --version "^2.0" --locked

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Project Structure

```
Drift/
├── src/               # Frontend (HTML/CSS/JS)
│   └── index.html
├── src-tauri/         # Rust backend
│   ├── src/
│   │   ├── main.rs
│   │   └── lib.rs
│   ├── icons/         # App icons (all sizes)
│   ├── Cargo.toml
│   ├── build.rs
│   └── tauri.conf.json
└── package.json
```
