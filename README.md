# Zed Prisma

A [Prisma](https://www.prisma.io/) extension for [Zed](https://zed.dev). Supports Prisma 8 (`prisma lsp`) and earlier versions (`@prisma/language-server`).

## Features

- **Prisma 8 Support**: Automatically detects Prisma 8 projects and runs `prisma lsp --stdio`.
- **Autocompletion & Hover**: Model, field, attribute completions and hover tooltips.
- **Syntax Highlighting**: Tree-sitter queries with support for Prisma 8 syntax (`mixin`, `contract`, `+Mixin`, namespaced types).
- **Legacy Fallback**: Uses `@prisma/language-server` for Prisma 2–7 projects.

## Configuration

In your Zed `settings.json`:

```json
{
  "lsp": {
    "prisma-language-server": {
      "settings": {
        "prisma": {
          // "auto" (default), "prisma8", or "legacy"
          "serverMode": "auto",
          // Pin legacy server to Prisma v6
          "pinToPrisma6": false,
          "enableDiagnostics": true
        }
      }
    }
  }
}
```

### Custom Binary

```json
{
  "lsp": {
    "prisma-language-server": {
      "binary": {
        "path": "/path/to/prisma",
        "arguments": ["lsp", "--stdio"]
      }
    }
  }
}
```

## Development

1. Ensure the Wasm target is installed: `rustup target add wasm32-wasip1`
2. Build the extension: `cargo build --release --target wasm32-wasip1`
3. In Zed, run `zed: install dev extension` from the Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`) and select this directory.
