use ansi_term::Colour::{Green, Red, White};
use anyhow::Result;
use rl_sys::readline::{self, redisplay};
use rl_sys::history::{listmgmt, mgmt};
use std::io::{self, Write};
use std::process;
use std::sync::mpsc::channel;
use std::thread;
use tungstenite::Message;
use url::Url;

pub fn wscat_client(url: Url, _auth_option: Option<String>) -> Result<()> {
    let (ws, _response) = tungstenite::connect(url)?;
    use std::sync::{Arc, Mutex};
    let ws = Arc::new(Mutex::new(ws));


    // channel for sending messages from readline to ws send thread
    let (tx, rx) = channel();

    let ws_1 = ws.clone();
    // Thread for sending to ws
    let send = thread::spawn(move || {
        loop {
            let message: Message = rx.recv().unwrap();
            println!("message received for writing to websocket");

            let mut ws = ws_1.lock().unwrap();
            println!("websocket unlocked for read");
            if let Err(err) = ws.write_message(message) {
                let out = format!("Connection Closed: {}", err);
                println!("");
                println!("{}", Red.paint(out));
                process::exit(1);
            }
        }
    });

    // Thread for receiving from ws
    let tx_1 = tx.clone();
    let ws_2 = ws.clone();
    let receive = thread::spawn(move || {
        loop {
            // ugh, this is ridiculous to have a mutex here. But I'm going to split when moving to
            // async anyways.
            let mut ws = ws_2.lock().unwrap();
            let message: Message = match ws.read_message() {
                Ok(m) => m,
                Err(err) => {
                    let out = format!("Connection Closed: {}", err);
                    println!("");
                    println!("{}", Red.paint(out));
                    process::exit(1);
                },
            };

            //write to stdout depending on opcode
            let out = match message {
                Message::Ping(payload) => {
                    tx_1.send(Message::Pong(payload)).unwrap();
                    format!("{}", Green.paint("Ping!\n")) //add color
                },
                Message::Text(payload) => {
                    let out = format!("<< {}\n", payload);
                    format!("{}", White.dimmed().paint(out))
                },
                Message::Binary(payload) => {
                    // Binary just supported as text here; no downloading, etc.
                    let out = format!("<< {}\n", String::from_utf8(payload).unwrap());
                    format!("{}", White.dimmed().paint(out))
                },
                Message::Close(_) => {
                    println!("");
                    let out = format!("{}", Red.paint("Connection Closed: Close message received"));
                    println!("{}", out);
                    process::exit(0);
                },
                _ => format!("Unsupported ws message"),
            };

            redisplay::save_prompt();

            //clear line, maybe there's easier way in readline
            // the 2K is to clear line completely
            // the 2D is to move cursor back two spaces (from where it is
            // after clearing the line, goes to original cursor position)
            // Hmm... something weird happened. Now I'm using 1G to move to
            // beginning of line. Not sure what changed from last version.
            let esc = String::from_utf8(vec![27]).unwrap();
            let clear_line_bytes = format!("{}[2K{}[1G", esc, esc).into_bytes();
            io::stdout().write(&clear_line_bytes).expect("error clearing line");

            io::stdout().write(&out.as_bytes()).unwrap();
            io::stdout().flush().unwrap();
            redisplay::on_new_line().unwrap();
            redisplay::rl_restore_prompt();
            redisplay::redisplay();
        }
    });

    loop {
        let input = match readline::readline("> ") {
            Ok(Some(i)) => i,
            Ok(None) => continue,
            _ => break,
        };
        listmgmt::add(&input).unwrap();
        let _ = tx.send(Message::text(input));
        println!("message sent for writing to websocket");
    }

    mgmt::cleanup();

    // unwrap which exits program
    send.join().unwrap();
    receive.join().unwrap();

    Ok(())
}



// TODO do this later
// refactor to use from_str
//pub fn parse_authorization(user_password: &str) -> Option<Authorization<Basic>> {
//    let v: Vec<_> = user_password.split(':').collect();
//    if v.len() > 2 {
//        None
//    } else {
//        Some(Authorization (
//            Basic {
//                username: v[0].to_owned(),
//                password: v.get(1).map(|&p| p.to_owned()),
//            }
//        ))
//    }
//}
