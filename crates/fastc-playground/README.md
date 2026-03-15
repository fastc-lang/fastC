# FastC Playground

Browser-based IDE and playground for FastC.

## Features

- Monaco Editor with FastC syntax highlighting
- Real-time compilation to C11
- WebSocket streaming for live output
- Terminal emulator for program execution
- Optional token authentication for API/WebSocket
- Per-IP run rate limiting and execution concurrency limits
- Request/code size limits and execution resource caps

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
      --auth-token <AUTH_TOKEN>
          Require this token for API and WebSocket access
      --allow-origins <ALLOW_ORIGINS>
          Comma-separated allowed CORS origins. Empty disables CORS
      --max-request-bytes <MAX_REQUEST_BYTES>
          Maximum HTTP request body size [default: 131072]
      --max-code-bytes <MAX_CODE_BYTES>
          Maximum FastC source size accepted by run/compile endpoints [default: 65536]
      --max-runs-per-minute <MAX_RUNS_PER_MINUTE>
          Maximum run requests allowed per IP per minute [default: 30]
      --max-concurrent-runs <MAX_CONCURRENT_RUNS>
          Maximum number of concurrent executions [default: 4]
      --run-timeout-secs <RUN_TIMEOUT_SECS>
          Execution timeout in seconds [default: 5]
      --compile-timeout-secs <COMPILE_TIMEOUT_SECS>
          Native C compilation timeout in seconds [default: 10]
      --max-output-bytes <MAX_OUTPUT_BYTES>
          Maximum process output bytes streamed to the client [default: 65536]
      --max-memory-mb <MAX_MEMORY_MB>
          Maximum address-space size for executed user binaries in MB [default: 256]
      --max-processes <MAX_PROCESSES>
          Maximum number of processes/threads for executed user binaries [default: 32]
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
