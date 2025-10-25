mod room;
mod state;
mod codegen;
mod server;
mod conn;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let listen = std::env::args().nth(1).unwrap_or_else(|| "0.0.0.0:7000".to_string());

    let state = state::ServerState::default();

    server::run(&listen, state).await
}