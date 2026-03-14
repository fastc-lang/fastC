# FastC Playground

Browser-based IDE and playground for FastC.

## Features

- Monaco Editor with FastC syntax highlighting
- Real-time compilation to C11
- WebSocket streaming for live output
- Terminal emulator for program execution
- Cookie-based session management

## Usage

```bash
# Build the frontend first
cd frontend
npm install
npm run build
cd ..

# Run the playground server
cargo run -p fastc-playground -- --port 3000

# Or with auto-open browser
cargo run -p fastc-playground -- --port 3000 --open
```

Then visit http://localhost:3000

## CLI Options

```
fastc-playground [OPTIONS]

Options:
  -p, --port <PORT>    Port to listen on [default: 3000]
  -H, --host <HOST>    Host to bind to [default: 127.0.0.1]
  -o, --open           Open browser automatically
  -h, --help           Print help
  -V, --version        Print version
```

## API Endpoints

- `POST /api/compile` - Compile FastC code to C
- `POST /api/check` - Type-check code without compiling
- `POST /api/format` - Format FastC code
- `POST /api/run` - Compile and execute code
- `WS /ws` - WebSocket for streaming output

## Development

```bash
# Run frontend dev server (with hot reload)
cd frontend
npm run dev

# Run backend (in another terminal)
cargo run -p fastc-playground
```

## License

This project is licensed under the [MIT License](../../LICENSE).
