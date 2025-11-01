use anyhow::Result;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

use crate::protocol::{Command, parse_command};
use crate::state::ServerState;
use crate::{codegen, room::Room};


struct ClientCtx {
    nick: Option<String>,
    room_code: Option<String>,
    room_rx: Option<broadcast::Receiver<String>>
}

impl ClientCtx {
    fn new() -> Self {
        Self {
            nick : None,
            room_code: None,
            room_rx: None
        }
    }
}

pub async fn handle(state: ServerState, socket: TcpStream, peer: SocketAddr) -> Result<()> {
    let (reader, mut writer) = socket.into_split();

    let mut lines = BufReader::new(reader).lines();

    let mut ctx = ClientCtx::new();

    writer
        .write_all(b"Welcome to Relay!\nType HELP for commands\n")
        .await?;


    loop {
        if let Some(rx) = &mut ctx.room_rx {

            tokio::select! {
                // Branch A: Room broadcast received
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            writer.write_all(msg.as_bytes()).await?;
                            writer.write_all(b"\n").await?;
                        }

                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            // Client is too slow, skipped messages
                            writer.write_all(
                                format!("[server] Warning: skipped {} message\n", n).as_bytes()
                            ).await?;
                        }

                        Err(broadcast::error::RecvError::Closed) => {
                            // Room channel closed (room was deleted)
                            ctx.room_rx = None;
                            writer.write_all(b"[server] Room closed\n").await?;
                        }
                    }
                }
                
                // Branch B: Client send a line
                line_result = lines.next_line() => {
                    match line_result {
                        Ok(Some(line)) => {
                            let line = line.trim();
                            if line.is_empty() {
                                continue;
                            }

                            let cmd = match parse_command(line) {
                                Ok(c) => c,
                                Err(e) => {
                                    writer.write_all(format!("[error] {}\n", e).as_bytes()).await?;
                                    continue;
                                }
                            };

                            if let Err(e) = handle_command(&state, &mut ctx, &mut writer, cmd).await {
                                writer.write_all(format!("[error] {}\n", e).as_bytes()).await?;
                            }
                        }

                        Ok(None) => {
                            // Client disconnected (EOF)
                            break;
                        }
                        Err(e) => {
                            return Err(e.into());
                        }
                    }
                }

            }
        } else {
            // Not in a room - only handle client input (no broadcasts)
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let cmd = match parse_command(line) {
                        Ok(c) => c,
                        Err(e) => {
                            writer.write_all(format!("[error] {}\n", e).as_bytes()).await?;
                            continue;
                        }
                    };

                    if let Err(e) = handle_command(&state, &mut ctx, &mut writer, cmd).await {
                        writer.write_all(format!("[error] {}\n", e).as_bytes()).await?;
                    }
                }

                Ok(None) => break,
                Err(e) => return Err(e.into())
            }
        }
    }

    // Cleanup on disconnect
    if let Some(code) = ctx.room_code.take() {
        if let Some(room) = state.get_room(&code) {
            room.dec();
            if let Some(nick) = &ctx.nick {
                room.send(format!("[server] {} left.", nick));
            }

            state.remove_if_empty(&code);
        }
    }

    eprintln!("[{}] disconnected", peer);

    Ok(())
}

async fn handle_command(
    state: &ServerState,
    ctx: &mut ClientCtx,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    cmd: crate::protocol::Command
) -> Result<(), String> {
    match cmd {
        Command::Help => {
            writer
                .write_all(
                    b"Commands:\n\
                      NICK <name>   - set your nickname\n\
                      CREATE        - create a new room\n\
                      JOIN <CODE>   - join an existing room\n\
                      MSG <text>    - send a message to your room\n\
                      QUIT          - disconnect\n",
                ).await
                .map_err(|e| e.to_string())?;
        }

        Command::Quit => {
            writer
                .write_all(b"Goodbye.\n")
                .await
                .map_err(|e| e.to_string())?;

            return Err("client quit".into());
        }

        Command::Nick(name) => {
            ctx.nick = Some(name.clone());
            writer
                .write_all(format!("[ok] nickname set to '{}'\n", name).as_bytes())
                .await
                .map_err(|e| e.to_string())?;
        }   

        Command::Create => {
            let nick = ctx.nick.clone().ok_or("set a nickname first: NICK <name>")?;

            let code = codegen::unique_code(state, codegen::CODE_LEN);

            let room = Room::new(512);

            state.insert_room(code.clone(), room.clone());

            room.inc();

            ctx.room_code = Some(code.clone());
            ctx.room_rx = Some(room.subscribe());

            room.send(format!("[server] {} joined", nick));

            writer
                .write_all(format!("[ok] room created: {}\n", code).as_bytes())
                .await
                .map_err(|e| e.to_string())?;
        }

        Command::Join(code) => {
            let nick = ctx.nick.clone().ok_or("set a nickname first: NICK <name>")?;

            let room = state
                .get_room(&code)
                .ok_or(format!("no such room: {}", code))?;
            
            // Leave old room if any
            if let Some(old_code) = ctx.room_code.take() {
                if let Some(old_room) = state.get_room(&old_code) {
                    old_room.dec();
                    old_room.send(format!("[server] {} left.", nick));
                    state.remove_if_empty(&old_code)
                }
                ctx.room_rx = None;
            }

            // Subscribe to new room broadcasts
            let rx = room.subscribe();
            ctx.room_rx = Some(rx);

            // Join new room
            room.inc();
            room.send(format!("[server] {} joined.", nick));
            ctx.room_code = Some(code.clone());

            writer 
                .write_all(format!("[ok] joined room '{}'\n", code).as_bytes())
                .await
                .map_err(|e| e.to_string())?;
        }

        Command::Msg(text) => {
            let nick = ctx.nick.clone().ok_or("set a nickname first: NICK <name>")?;
            let code = ctx.room_code.as_ref().ok_or("join a room first: JOIN <CODE>")?;

            let room = state
                .get_room(code)
                .ok_or("room no longer exists")?;

            room.send(format!("{}: {}", nick, text));
        }
    }

    Ok(())
}