use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    Unauthenticated,
    UsernameUnavailable,
    UsernameAccepted,
    Message(Message),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    username: String,
    message: String,
}

impl Message {
    pub fn new(u: String, m: String) -> Message {
        Message {
            username: u,
            message: m,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Username(String),
    Message(String),
}
