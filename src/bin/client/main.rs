use anyhow::{bail, Result};
use chrono::{Local, Timelike};
use crossterm::{
    cursor::MoveUp,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use rust_chat_app::{ClientMessage, ServerMessage};
use std::io::{prelude::BufRead, stdin, BufReader, Write};
use std::io::{stdout, Stdout};
use std::net::TcpStream;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() -> Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:3000")?;
    let mut stdout = stdout();
    let stream_clone = stream.try_clone().expect("Could not clone stream");
    let is_auth = Arc::new(Mutex::new(false));

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

    let is_auth_c = Arc::clone(&is_auth);
    thread::spawn(move || {
        while let Ok(user_input) = capture_input(&mut stdout) {
            if *is_auth_c.lock().unwrap() {
                send_message(&mut stream, user_input)?;
            } else {
                send_username(&mut stream, user_input)?;
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            ServerMessage::Message(m) => {
                let now = Local::now();
                println!(
                    "{:02}:{:02} {}: {}",
                    now.hour(),
                    now.minute(),
                    m.username,
                    m.message
                );
            }
            ServerMessage::UsernameAccepted => {
                *is_auth.lock().unwrap() = true;
                println!("Username accepted!");
            }
            ServerMessage::UsernameUnavailable => {
                *is_auth.lock().unwrap() = false;
                eprintln!("Username rejected. Try again.");
            }
            ServerMessage::Unauthenticated => {
                *is_auth.lock().unwrap() = false;
                eprintln!("You are unathenticated. Enter a username to authenticate yourself.");
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

fn capture_input(stdout: &mut Stdout) -> Result<String> {
    let mut input_string = String::new();
    stdin().read_line(&mut input_string)?;
    let trimmed = input_string.trim();

    // clear the line just entered
    stdout.execute(MoveUp(1))?;
    stdout.execute(Clear(ClearType::CurrentLine))?;

    if trimmed.is_empty() {
        bail!("input string empty")
    } else {
        Ok(trimmed.to_string())
    }
}
