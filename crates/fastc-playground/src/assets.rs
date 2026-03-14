use rust_embed::Embed;

/// Embedded static assets from the frontend build
#[derive(Embed)]
#[folder = "frontend/dist/"]
pub struct Assets;
