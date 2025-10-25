use anyhow::Result;
use tokio::net::TcpListener;

use crate::state::ServerState;

pub async fn run(listen_addr: &str, state: ServerState) -> Result<()> {
    let listener = TcpListener::bind(listen_addr).await?;

    eprintln!("listening on {}", listen_addr);

    loop {
        let (socket, peer) = listener.accept().await?;

        let state = state.clone();

        tokio::spawn(async move {
            if let Err(err) = crate::conn::handle(state, socket, peer).await {
                eprintln!("[{}] connection error: {err:?}", peer);
            }
        });
    }
}