use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    Unauthenticated,
    UsernameUnavailable,
    UsernameAccepted,
    Message(Message),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub username: String,
    pub message: String,
}

impl Message {
    pub fn new(u: String, m: String) -> Message {
        Message {
            username: u,
            message: m,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Username(String),
    Message(String),
}
