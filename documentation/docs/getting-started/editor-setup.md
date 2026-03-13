# Editor Setup

Configure your editor for the best FastC development experience.

## VS Code

### Language Server (Recommended)

FastC includes a Language Server Protocol (LSP) implementation for rich editor support.

#### Building the LSP

```bash
cd fastc
cargo build --release -p fastc-lsp
```

The binary is at `target/release/fastc-lsp`.

#### VS Code Configuration

Create or edit `.vscode/settings.json` in your workspace:

```json
{
    "fastc.serverPath": "/path/to/fastc/target/release/fastc-lsp"
}
```

### Syntax Highlighting

FastC syntax is similar to C. You can use C syntax highlighting as a fallback:

1. Open a `.fc` file
2. Click the language mode in the status bar (bottom right)
3. Select "Configure File Association for '.fc'"
4. Choose "C"

### Recommended Extensions

- **C/C++** by Microsoft - Provides C syntax highlighting
- **Error Lens** - Inline error display

## Vim/Neovim

### Syntax Highlighting

Add to your `.vimrc` or `init.vim`:

```vim
" Treat .fc files as C for syntax highlighting
autocmd BufRead,BufNewFile *.fc set filetype=c
```

### LSP with nvim-lspconfig

If using Neovim with nvim-lspconfig:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define FastC LSP
if not configs.fastc then
  configs.fastc = {
    default_config = {
      cmd = { '/path/to/fastc-lsp' },
      filetypes = { 'fastc' },
      root_dir = lspconfig.util.root_pattern('fastc.toml', '.git'),
    },
  }
end

lspconfig.fastc.setup{}
```

Add filetype detection in `~/.config/nvim/ftdetect/fastc.vim`:

```vim
autocmd BufRead,BufNewFile *.fc set filetype=fastc
```

## Emacs

### Basic Setup

Add to your Emacs config:

```elisp
;; Associate .fc files with c-mode for syntax highlighting
(add-to-list 'auto-mode-alist '("\\.fc\\'" . c-mode))
```

### With eglot (LSP)

```elisp
(require 'eglot)
(add-to-list 'eglot-server-programs
             '(c-mode . ("/path/to/fastc-lsp")))
```

## Sublime Text

### Syntax Highlighting

1. Open a `.fc` file
2. Go to View > Syntax > Open all with current extension as... > C

### Build System

Create `FastC.sublime-build`:

```json
{
    "cmd": ["fastc", "check", "$file"],
    "selector": "source.c",
    "file_patterns": ["*.fc"],
    "working_dir": "$file_path"
}
```

## Command Line Workflow

If you prefer minimal editor setup, use the CLI tools:

```bash
# Check for errors
fastc check src/main.fc

# Format code
fastc fmt src/main.fc

# Build and run
fastc run
```

### Watch Mode (with external tools)

Use `watchexec` or `entr` for auto-rebuild:

```bash
# Using watchexec
watchexec -e fc -- fastc check src/main.fc

# Using entr
find src -name '*.fc' | entr fastc check src/main.fc
```

## LSP Features

The FastC language server provides:

- **Diagnostics** - Real-time error reporting
- **Go to Definition** - Jump to function/type definitions
- **Hover Information** - Type information on hover
- **Document Symbols** - Outline view of functions and types

## Tips

1. **Save frequently** - The LSP checks on save
2. **Use fastc check** - Quick validation without full compilation
3. **Format on save** - Configure your editor to run `fastc fmt`

## Troubleshooting

### LSP not connecting

1. Check the server path is correct
2. Ensure the binary has execute permissions
3. Check editor's LSP logs for errors

### No syntax highlighting

Ensure `.fc` files are associated with C syntax as a fallback.

### Errors not showing

Run `fastc check` manually to verify the file has errors. Some editors need explicit save to trigger diagnostics.
