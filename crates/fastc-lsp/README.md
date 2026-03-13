# FastC Language Server

Language Server Protocol (LSP) implementation for FastC.

## Features

- **Diagnostics** - Real-time error and warning reporting
- **Go to Definition** - Navigate to function and type definitions
- **Hover** - Type information on hover
- **Workspace Support** - Multi-file project support

## Installation

```bash
# Build from source
cargo install --path .

# Or build the workspace
cargo build --release -p fastc-lsp
```

## Editor Setup

### VS Code

Install the FastC extension or configure manually:

```json
{
  "fastc.lsp.path": "/path/to/fastc-lsp"
}
```

### Neovim (nvim-lspconfig)

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

configs.fastc = {
  default_config = {
    cmd = { 'fastc-lsp' },
    filetypes = { 'fastc' },
    root_dir = lspconfig.util.root_pattern('fastc.toml', '.git'),
  },
}

lspconfig.fastc.setup{}
```

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "fastc"
scope = "source.fastc"
file-types = ["fc"]
language-servers = ["fastc-lsp"]

[language-server.fastc-lsp]
command = "fastc-lsp"
```

## Usage

The language server is typically started automatically by your editor. For manual testing:

```bash
# Start the server (communicates via stdio)
fastc-lsp
```

## Documentation

- [Editor Setup Guide](https://docs.skelfresearch.com/fastc/getting-started/editor-setup/)
- [FastC Documentation](https://docs.skelfresearch.com/fastc)

## License

This project is licensed under the [MIT License](../../LICENSE).
