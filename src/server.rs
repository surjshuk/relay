//! TCP server module for accepting and spawning client connections.
//!
//! This module contains the main server loop that:
//! - Binds to a network address
//! - Accepts incoming client connections
//! - Spawns a concurrent task for each client

use anyhow::Result;
use tokio::net::TcpListener;

use crate::state::ServerState;

/// Start the TCP server and accept connections.
///
/// This function creates a listening socket (the "front door") that waits for
/// incoming client connections. For each client that connects, it spawns a
/// new asynchronous task to handle that client independently.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────┐
/// │   Your Server Process   │
/// │                         │
/// │  ┌───────────────────┐  │
/// │  │   TcpListener     │  │ ← One per server (the "front door")
/// │  │  (listening on    │  │
/// │  │   0.0.0.0:7000)   │  │
/// │  └────────┬──────────┘  │
/// │           │              │
/// │           │ .accept()    │
/// │           │              │
/// │           ├──> TcpStream (alice) ← socket 1
/// │           │    127.0.0.1:54321
/// │           │
/// │           ├──> TcpStream (bob)   ← socket 2
/// │           │    127.0.0.1:54322
/// │           │
/// │           └──> TcpStream (carol) ← socket 3
/// │                127.0.0.1:54323
/// └─────────────────────────┘
/// ```
///
/// # Arguments
///
/// * `listen_addr` - The server's address to bind to (e.g., `"0.0.0.0:7000"`)
///   - `0.0.0.0` means "listen on all network interfaces"
///   - `127.0.0.1` means "listen only on localhost"
/// * `state` - Shared server state containing the room registry
///
/// # Returns
///
/// Returns `Ok(())` on successful shutdown (never happens in normal operation),
/// or an error if the server fails to bind or accept connections.
///
/// # Errors
///
/// This function will return an error if:
/// - The address is already in use (another process is using the port)
/// - Insufficient permissions to bind to the port (e.g., ports < 1024 on Unix)
/// - Network interface is unavailable
///
/// # Example
///
/// ```no_run
/// use relay::state::ServerState;
/// use relay::server;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let state = ServerState::default();
///     server::run("0.0.0.0:7000", state).await
/// }
/// ```
///
/// # Concurrency Model
///
/// Each client connection is handled in its own spawned task:
/// ```text
/// Main loop (run):           Spawned tasks (one per client):
///   ┌──────────┐               ┌──────────┐
///   │ accept() │──spawns────>  │ handle() │ (alice)
///   │          │               └──────────┘
///   │ accept() │──spawns────>  ┌──────────┐
///   │          │               │ handle() │ (bob)
///   │ accept() │               └──────────┘
///   │   ...    │
///   └──────────┘
/// ```

pub async fn run(listen_addr: &str, state: ServerState) -> Result<()> {
    // Create the listening socket (TcpListener) - the "front door"
    // This reserves the port with the OS and prepares to accept connections
    let listener = TcpListener::bind(listen_addr).await?;

    eprintln!("listening on {}", listen_addr);

    // Accept connections forever
    loop {
        // Wait for the next client to connect
        // Returns:
        //   - socket: TcpStream for this specific client (Connection Socket)
        //   - peer: client's IP address and port (e.g., "127.0.0.1:54321")
        let (socket, peer) = listener.accept().await?;

        // Clone the shared state (cheap - only clones Arc pointers)
        // Each task needs its own handle to the shared state
        let state = state.clone();

        // Spawn a new concurrent task to handle this client
        // The task runs independently - the main loop immediately
        // goes back to accepting the next client
        tokio::spawn(async move {
            // Handle this client's connection
            // If an error occurs, log it but don't crash the server
            if let Err(err) = crate::conn::handle(state, socket, peer).await {
                eprintln!("[{}] connection error: {err:?}", peer);
            }
        });

        // Loop continues immediately - listener is still open and ready
        // for the next client to connect
    }
}