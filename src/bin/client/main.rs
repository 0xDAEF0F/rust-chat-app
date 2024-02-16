use anyhow::Result;
use chrono::{Local, Timelike};
use std::io::prelude::BufRead;
use std::io::{stdin, BufReader, Write};
use std::net::TcpStream;
use std::thread;

fn main() -> Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:3000")?;
    let stream_clone = stream.try_clone().expect("Could not clone stream");

    let reader_handle = thread::spawn(move || {
        let reader = BufReader::new(stream_clone);
        for line in reader.lines() {
            match line {
                Ok(msg) => {
                    let now = Local::now();
                    println!("{}:{} — {}", now.hour(), now.minute(), msg);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    });

    loop {
        let mut input_string = String::new();

        stdin()
            .read_line(&mut input_string)
            .expect("Failed to read line");

        match input_string.trim() {
            "cancel" => break,
            val => match val.is_ascii() && !val.is_empty() {
                false => break,
                true => {
                    stream.write_all(format!("{val}\n").as_bytes()).unwrap();
                    stream.flush().unwrap();
                }
            },
        }
    }

    reader_handle.join().unwrap();

    Ok(())
}
