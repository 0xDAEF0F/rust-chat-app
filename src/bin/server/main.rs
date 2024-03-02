use anyhow::Result;
use rust_chat_app::ClientMessage;
use state::{Peer, ServerState};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

mod state;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:3000").await?;

    let senders = Arc::new(Mutex::new(ServerState::new()));

    loop {
        println!("loop");
        match listener.accept().await {
            Ok((stream, addr)) => {
                let server_state = Arc::clone(&senders);
                println!("hello");
                tokio::spawn(async move { handle_client(stream, server_state, addr).await });
                println!("world");
            }
            Err(e) => eprintln!("Failed to accept client: {}", e),
        };
    }
}

async fn handle_client(
    stream: TcpStream,
    server_state: Arc<Mutex<ServerState>>,
    peer_addr: SocketAddr,
) {
    println!("new peer connected: {:?}", peer_addr);
    let state_clone = Arc::clone(&server_state);

    let (reader, mut writer) = stream.into_split();

    let (tx, mut rx) = mpsc::channel(100);

    let reader_handle = tokio::spawn(async move {
        let reader = BufReader::new(reader);
        let server_state = server_state;

        let mut lines = reader.lines();

        state_clone
            .lock()
            .unwrap()
            .add_peer(Peer::new(tx, peer_addr, None))
            .request_authentication()
            .unwrap();

        loop {
            match lines.next_line().await {
                Ok(line) => {
                    if line.is_none() {
                        break;
                    }
                    let line = line.unwrap();
                    let client_message: ClientMessage = serde_json::from_str(&line)?;
                    let mut state = server_state.lock().unwrap();
                    match client_message {
                        ClientMessage::Message(m) => {
                            if state.is_peer_authenticated(&peer_addr)? {
                                let msg = state.build_peer_message(&peer_addr, m)?;
                                state.cast_message(msg);
                            } else {
                                state.peer_unauthenticated(&peer_addr)?;
                            }
                        }
                        ClientMessage::Username(u) => {
                            if state.is_username_available(&u) {
                                state.register_username(&peer_addr, u)?;
                            } else {
                                state.username_unavailable(&peer_addr)?;
                            }
                        }
                    }
                }
                Err(e) => eprintln!("could not read line: {:?}", e),
            }
        }

        server_state.lock().unwrap().remove_peer(peer_addr);

        Ok::<(), anyhow::Error>(())
    });

    let writer_handle = tokio::spawn(async move {
        loop {
            let msg = rx.recv().await;
            if msg.is_none() {
                break;
            }
            let msg = msg.unwrap();
            let s_msg = serde_json::to_string(&msg).unwrap();
            let _ = writer.write_all(format!("{}\n", s_msg).as_bytes()).await;
            let _ = writer.flush().await;
        }
    });

    let _ = tokio::join!(reader_handle, writer_handle);

    println!("dropping the thread of peer: {}", peer_addr);
}
