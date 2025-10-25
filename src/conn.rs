use anyhow::Result;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::state::ServerState;

pub async fn handle(state: ServerState, socket: TcpStream, peer: SocketAddr) -> Result<()> {
    let (reader, mut writer) = socket.into_split();

    let mut lines = BufReader::new(reader).lines();

    let _ = state;

    writer
        .write_all(b"Welcome to Relay!\nType HELP for commands.\n")
        .await?;

        while let Some(line) = lines.next_line().await? {
            let line = line.trim();

            if line.eq_ignore_ascii_case("quit") {
                writer.write_all(b"Goodbye.\n").await?;
                break;
            }

            writer.write_all(b"you said: ").await?;
            writer.write_all(line.as_bytes()).await?;
            writer.write_all(b"\n").await?;
        }

        eprintln!("[{}] disconnected", peer);
        Ok(())
}