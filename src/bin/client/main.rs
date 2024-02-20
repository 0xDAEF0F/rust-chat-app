use anyhow::{bail, Result};
use chrono::{Local, Timelike};
use rust_chat_app::{ClientMessage, ServerMessage};
use std::io::{prelude::BufRead, stdin, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;

fn main() -> Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:3000")?;
    let stream_clone = stream.try_clone().expect("Could not clone stream");

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let reader = BufReader::new(stream_clone);
        let tx = tx;

        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let server_msg: ServerMessage = serde_json::from_str(&l)?;
                    tx.send(server_msg)?;
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            ServerMessage::Message(m) => {
                let now = Local::now();
                println!(
                    "{:2}:{:2} {}: {}",
                    now.hour(),
                    now.minute(),
                    m.username,
                    m.message
                );
                send_message(&mut stream, capture_input()?)?;
            }
            ServerMessage::UsernameAccepted => {
                println!("Username accepted!");
                send_message(&mut stream, capture_input()?)?;
            }
            ServerMessage::UsernameUnavailable => {
                eprintln!("Username rejected. Try again.");
                send_username(&mut stream, capture_input()?)?;
            }
            ServerMessage::Unauthenticated => {
                eprintln!("You are unathenticated. Enter a username to authenticate yourself.");
                send_username(&mut stream, capture_input()?)?;
            }
        }
    }

    Ok(())
}

fn send_username(stream: &mut TcpStream, username: String) -> Result<()> {
    let client_message = ClientMessage::Username(username);
    let client_message = serde_json::to_string(&client_message)?;

    stream.write_all(format!("{client_message}\n").as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn send_message(stream: &mut TcpStream, message: String) -> Result<()> {
    let client_message = ClientMessage::Message(message);
    let client_message = serde_json::to_string(&client_message)?;

    stream.write_all(format!("{client_message}\n").as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn capture_input() -> Result<String> {
    let mut input_string = String::new();
    stdin().read_line(&mut input_string)?;
    let trimmed = input_string.trim();

    if trimmed.is_empty() {
        bail!("input string empty")
    } else {
        Ok(trimmed.to_string())
    }
}
