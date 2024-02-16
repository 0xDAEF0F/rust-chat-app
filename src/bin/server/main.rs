use anyhow::Result;
use std::collections::HashMap;
use std::io::prelude::BufRead;
use std::io::{BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

struct Client {
    transmitter: mpsc::Sender<String>,
    client_addr: SocketAddr,
}

struct SharedState {
    clients: Vec<Client>,
    position_in_list: HashMap<SocketAddr, usize>,
}

impl SharedState {
    fn new() -> Self {
        Self {
            clients: vec![],
            position_in_list: HashMap::new(),
        }
    }

    fn publish_message(&mut self, msg: String) {
        for sender in self.clients.iter() {
            match sender.transmitter.send(msg.clone()) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }

    fn add_client(&mut self, client: Client) {
        self.position_in_list
            .insert(client.client_addr, self.clients.len());
        self.clients.push(client);
    }

    fn remove_client(&mut self, socket_addr: SocketAddr) {
        println!("Removing peer: {}", socket_addr);

        let client_position = *(self.position_in_list.get(&socket_addr).unwrap());
        let last_client_addr = self.clients.last().unwrap().client_addr;

        // update the last client position to the one of the old
        self.position_in_list
            .insert(last_client_addr, client_position);

        // remove the client_position and it will be replaced by the last one
        self.clients.swap_remove(client_position);

        println!("length of list: {}", self.clients.len());
    }
}

fn handle_client(stream: TcpStream, senders: Arc<Mutex<SharedState>>) {
    let peer_addr = stream.peer_addr().unwrap();
    println!("new peer: {:?}", peer_addr);

    let reader = stream.try_clone().expect("Could not clone stream");
    let mut writer = stream;

    let (tx, rx) = mpsc::channel();

    senders.lock().unwrap().add_client(Client {
        transmitter: tx,
        client_addr: peer_addr,
    });

    let reader_handle = thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    println!("msg: {}", line);
                    senders.lock().unwrap().publish_message(line);
                }
                Err(_) => {}
            }
        }
        senders.lock().unwrap().remove_client(peer_addr);
    });

    let writer_handle = thread::spawn(move || {
        for msg in rx {
            println!("will write msg: {}", msg);
            writer.write_all(format!("{}\n", msg).as_bytes()).unwrap();
            writer.flush().unwrap();
        }
    });

    let _ = reader_handle.join();

    let _ = writer_handle.join();

    println!("dropping the thread of peer: {}", peer_addr);
}

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:3000")?;

    let senders = Arc::new(Mutex::new(SharedState::new()));

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
