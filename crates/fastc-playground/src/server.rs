use crate::assets::Assets;
use crate::routes::{api, ws};
use crate::session::SessionStore;
use axum::{
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use rust_embed::Embed;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<SessionStore>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(SessionStore::new()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the playground server
pub async fn run_server(addr: SocketAddr) -> anyhow::Result<()> {
    let state = AppState::new();

    // Start session cleanup task
    let sessions = state.sessions.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            sessions.cleanup_expired(std::time::Duration::from_secs(3600));
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // API routes
        .route("/api/compile", post(api::compile))
        .route("/api/check", post(api::check))
        .route("/api/format", post(api::format))
        .route("/api/run", post(api::run))
        // WebSocket
        .route("/ws", get(ws::websocket_handler))
        // Static files and SPA fallback
        .route("/", get(index_handler))
        .route("/assets/*path", get(static_handler))
        .fallback(get(index_handler))
        .layer(cors)
        .with_state(state);

    tracing::info!("FastC Playground listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Serve the index.html
async fn index_handler() -> impl IntoResponse {
    match <Assets as Embed>::get("index.html") {
        Some(content) => Html(content.data.to_vec()).into_response(),
        None => (StatusCode::OK, Html(fallback_html())).into_response(),
    }
}

/// Serve static files
async fn static_handler(uri: Uri) -> impl IntoResponse {
    // The path includes /assets/ prefix, and files are stored as assets/filename in rust-embed
    let path = uri.path().trim_start_matches('/');

    match <Assets as Embed>::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data.to_vec()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Fallback HTML when frontend is not built
fn fallback_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FastC Playground</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #1e1e1e;
            color: #d4d4d4;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
        }
        .header {
            background: #252526;
            padding: 12px 20px;
            border-bottom: 1px solid #3c3c3c;
            display: flex;
            align-items: center;
            gap: 20px;
        }
        .header h1 { font-size: 18px; font-weight: 600; }
        .toolbar { display: flex; gap: 10px; }
        .btn {
            background: #0e639c;
            color: white;
            border: none;
            padding: 8px 16px;
            border-radius: 4px;
            cursor: pointer;
            font-size: 14px;
        }
        .btn:hover { background: #1177bb; }
        .btn:disabled { background: #3c3c3c; cursor: not-allowed; }
        .btn-run { background: #388a34; }
        .btn-run:hover { background: #43a83f; }
        .main {
            flex: 1;
            display: grid;
            grid-template-columns: 1fr 1fr;
            grid-template-rows: 1fr 200px;
            gap: 1px;
            background: #3c3c3c;
        }
        .panel {
            background: #1e1e1e;
            display: flex;
            flex-direction: column;
        }
        .panel-header {
            background: #252526;
            padding: 8px 12px;
            font-size: 12px;
            text-transform: uppercase;
            color: #888;
            border-bottom: 1px solid #3c3c3c;
        }
        .panel-content {
            flex: 1;
            padding: 12px;
            overflow: auto;
        }
        textarea, pre {
            width: 100%;
            height: 100%;
            background: transparent;
            color: #d4d4d4;
            border: none;
            font-family: 'Fira Code', 'Consolas', monospace;
            font-size: 14px;
            line-height: 1.5;
            resize: none;
            outline: none;
        }
        pre { white-space: pre-wrap; word-wrap: break-word; }
        .terminal {
            grid-column: 1 / -1;
            background: #0c0c0c;
        }
        .terminal pre { color: #33ff33; }
        .error { color: #f48771; }
        .success { color: #89d185; }
    </style>
</head>
<body>
    <div class="header">
        <h1>FastC Playground</h1>
        <div class="toolbar">
            <button class="btn" onclick="compile()">Compile</button>
            <button class="btn btn-run" onclick="run()">Run</button>
        </div>
    </div>
    <div class="main">
        <div class="panel">
            <div class="panel-header">FastC Code</div>
            <div class="panel-content">
                <textarea id="editor" spellcheck="false">// Welcome to FastC Playground!
// A safe C-like language that compiles to C11.

fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> i32 {
    let result: i32 = fibonacci(10);
    // Result: 55
    return result;
}</textarea>
            </div>
        </div>
        <div class="panel">
            <div class="panel-header">Generated C</div>
            <div class="panel-content">
                <pre id="output"></pre>
            </div>
        </div>
        <div class="panel terminal">
            <div class="panel-header">Terminal</div>
            <div class="panel-content">
                <pre id="terminal"></pre>
            </div>
        </div>
    </div>
    <script>
        const editor = document.getElementById('editor');
        const output = document.getElementById('output');
        const terminal = document.getElementById('terminal');
        let ws = null;

        function connectWs() {
            const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
            ws = new WebSocket(`${protocol}//${location.host}/ws`);
            ws.onmessage = (event) => {
                const msg = JSON.parse(event.data);
                if (msg.type === 'stdout' || msg.type === 'stderr') {
                    terminal.textContent += msg.data;
                } else if (msg.type === 'exit') {
                    terminal.innerHTML += `<span class="${msg.code === 0 ? 'success' : 'error'}">Process exited with code ${msg.code}</span>\n`;
                } else if (msg.type === 'error') {
                    terminal.innerHTML += `<span class="error">${msg.message}</span>\n`;
                } else if (msg.type === 'compile') {
                    terminal.textContent += msg.output + '\n';
                }
                terminal.scrollTop = terminal.scrollHeight;
            };
            ws.onclose = () => setTimeout(connectWs, 1000);
        }
        connectWs();

        async function compile() {
            output.textContent = 'Compiling...';
            try {
                const res = await fetch('/api/compile', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ code: editor.value, emit_header: false })
                });
                const data = await res.json();
                if (data.success) {
                    output.textContent = data.c_code;
                    output.className = '';
                } else {
                    output.textContent = data.error.message;
                    output.className = 'error';
                }
            } catch (e) {
                output.textContent = 'Error: ' + e.message;
                output.className = 'error';
            }
        }

        async function run() {
            terminal.textContent = '';
            output.textContent = 'Compiling and running...';
            try {
                const res = await fetch('/api/run', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ code: editor.value })
                });
                const data = await res.json();
                if (data.success) {
                    output.textContent = data.c_code || 'Running...';
                    if (ws && ws.readyState === WebSocket.OPEN) {
                        ws.send(JSON.stringify({ type: 'subscribe', session_id: data.session_id }));
                    }
                } else {
                    output.textContent = data.error.message;
                    output.className = 'error';
                }
            } catch (e) {
                output.textContent = 'Error: ' + e.message;
                output.className = 'error';
            }
        }

        // Keyboard shortcuts
        editor.addEventListener('keydown', (e) => {
            if (e.ctrlKey && e.key === 'Enter') {
                e.preventDefault();
                run();
            } else if (e.ctrlKey && e.key === 's') {
                e.preventDefault();
                compile();
            }
        });
    </script>
</body>
</html>"#.to_string()
}
