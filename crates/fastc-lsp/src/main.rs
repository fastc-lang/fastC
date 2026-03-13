//! FastC Language Server Protocol implementation

mod diagnostics;
mod server;
mod workspace;

use server::FastcLanguageServer;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(FastcLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
