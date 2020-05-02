use ansi_term::Colour::{Blue, Red, White};
use anyhow::Result;
use std::net::TcpListener;
use std::thread;
use tungstenite::Message;

pub fn wscat_server(port: usize) -> Result<()> {
    let out_port = format!("Listening on port {:?}", port);
    println!("{}", Blue.bold().paint(out_port));
    let url = format!("127.0.0.1:{}", port); 

    let listener = TcpListener::bind(&url.as_str())?;

    let mut handles = Vec::new();
    for stream in listener.incoming() {
        let stream = stream?;
        let mut ws = tungstenite::accept(stream)?;

        let handle = thread::spawn(move || {
            loop {
                let message: Message = match ws.read_message() {
                    Ok(m) => m,
                    Err(_) => {
                        let out = format!("Disconnecting ws");
                        println!("{}", Red.paint(out));
                        break;
                    }
                };

                match message {
                    Message::Close(_) => {
                        let message = Message::Close(None);
                        ws.write_message(message).unwrap();
                        println!("Client disconnected");
                        return;
                    },
                    Message::Ping(payload) => {
                        let message = Message::Pong(payload);
                        ws.write_message(message).unwrap();
                    },
                    Message::Pong(_) => {},
                    // Don't support binary
                    Message::Binary(_) => {},
                    Message::Text(payload) => {
                        println!("<< {} {}",
                            payload,
                            White.dimmed().paint(format!("(ip)"))
                        );
                    }
                }
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}

