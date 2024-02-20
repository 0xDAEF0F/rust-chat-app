use anyhow::Context;
use rust_chat_app::{Message, ServerMessage};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc;

pub struct ServerState {
    clients: Vec<Peer>,
    position_in_list: HashMap<SocketAddr, usize>,
}

impl ServerState {
    pub fn new() -> ServerState {
        ServerState {
            clients: vec![],
            position_in_list: HashMap::new(),
        }
    }

    pub fn is_username_available(&self, username: &str) -> bool {
        for peer in self.clients.iter() {
            if let Some(u) = &peer.username {
                if u == username {
                    return false;
                }
            }
        }
        true
    }

    pub fn cast_message(&self, msg: Message) {
        for sender in self.clients.iter() {
            let msg_c = msg.clone();

            match sender.transmitter.send(ServerMessage::Message(msg_c)) {
                Ok(_) => continue,
                Err(_) => eprintln!("could not deliver msg: {:?}", &msg),
            };
        }
    }

    pub fn build_peer_message(
        &self,
        peer_addr: &SocketAddr,
        msg: String,
    ) -> anyhow::Result<Message> {
        let idx = self
            .position_in_list
            .get(peer_addr)
            .context("peer should be in list")?;

        let username = self.clients[*idx]
            .username
            .clone()
            .context("username should exist")?;

        Ok(Message::new(username, msg))
    }

    pub fn is_peer_authenticated(&self, socket_addr: &SocketAddr) -> anyhow::Result<bool> {
        let idx = self
            .position_in_list
            .get(socket_addr)
            .context("peer should be in list")?;

        Ok(self.clients[*idx].username.is_some())
    }

    pub fn peer_unauthenticated(&self, peer: &SocketAddr) -> anyhow::Result<()> {
        let idx = self
            .position_in_list
            .get(peer)
            .context("peer should be in list")?;

        let tx = &self.clients[*idx].transmitter;

        tx.send(ServerMessage::Unauthenticated)
            .context("could not send message to rx")
    }

    pub fn username_unavailable(&self, peer: &SocketAddr) -> anyhow::Result<()> {
        let idx = self
            .position_in_list
            .get(peer)
            .context("peer should be in list")?;

        let tx = &self.clients[*idx].transmitter;

        tx.send(ServerMessage::UsernameUnavailable)
            .context("could not send message to rx")
    }

    pub fn add_peer(&mut self, client: Peer) -> &Peer {
        self.position_in_list
            .insert(client.peer_addr, self.clients.len());

        self.clients.push(client);

        self.clients.last().unwrap()
    }

    pub fn register_username(
        &mut self,
        peer_addr: &SocketAddr,
        username: String,
    ) -> anyhow::Result<()> {
        let idx = self
            .position_in_list
            .get(peer_addr)
            .context("peer should be in list")?;

        self.clients[*idx].username = Some(username);

        let tx = &self.clients[*idx].transmitter;

        tx.send(ServerMessage::UsernameAccepted)?;

        Ok(())
    }

    pub fn remove_peer(&mut self, socket_addr: SocketAddr) {
        println!("Removing peer: {}", socket_addr);

        let client_position = *(self.position_in_list.get(&socket_addr).unwrap());
        let last_client_addr = self.clients.last().unwrap().peer_addr;

        // update the last client position to the one of the old
        self.position_in_list
            .insert(last_client_addr, client_position);

        // remove the client_position and it will be replaced by the last one
        self.clients.swap_remove(client_position);

        println!("length of list: {}", self.clients.len());
    }
}

pub struct Peer {
    transmitter: mpsc::Sender<ServerMessage>,
    peer_addr: SocketAddr,
    username: Option<String>,
}

impl Peer {
    pub fn new(
        transmitter: mpsc::Sender<ServerMessage>,
        peer_addr: SocketAddr,
        username: Option<String>,
    ) -> Peer {
        Peer {
            transmitter,
            peer_addr,
            username,
        }
    }

    pub fn request_authentication(&self) -> anyhow::Result<()> {
        let tx = &self.transmitter;
        Ok(tx.send(ServerMessage::Unauthenticated)?)
    }
}
