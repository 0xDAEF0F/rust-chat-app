use anyhow::Result;
use rust_chat_app::ClientMessage;
use state::{Peer, ServerState};
use std::io::prelude::BufRead;
use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

mod state;

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:3000")?;

    let senders = Arc::new(Mutex::new(ServerState::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let senders = Arc::clone(&senders);
                thread::spawn(move || {
                    handle_client(stream, senders);
                });
            }
            Err(e) => eprintln!("Failed to accept client: {}", e),
        }
    }

    Ok(())
}

fn handle_client(stream: TcpStream, server_state: Arc<Mutex<ServerState>>) {
    let peer_addr = stream.peer_addr().unwrap();
    println!("new peer connected: {:?}", peer_addr);

    let reader = stream.try_clone().expect("Could not clone stream");
    let mut writer = stream;

    let (tx, rx) = mpsc::channel();

    server_state
        .lock()
        .unwrap()
        .add_peer(Peer::new(tx, peer_addr, None))
        .request_authentication()
        .unwrap();

    let reader_handle = thread::spawn(move || {
        let reader = BufReader::new(reader);

        for line in reader.lines() {
            match line {
                Ok(line) => {
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

    let writer_handle = thread::spawn(move || {
        for msg in rx {
            let s_msg = serde_json::to_string(&msg).unwrap();
            writer.write_all(format!("{}\n", s_msg).as_bytes()).unwrap();
            writer.flush().unwrap();
        }
    });

    let _ = reader_handle.join();
    let _ = writer_handle.join();

    println!("dropping the thread of peer: {}", peer_addr);
}
