use std::io::{self, Write};
use std::net::TcpListener;
use std::process;
use std::sync::mpsc::channel;
use std::thread;

use anyhow::{Context as _, Result};
use ansi_term::Colour::{Blue, Green, Red, White};
use clap::{crate_version, App, AppSettings, Arg, SubCommand};
use rl_sys::readline::{self, redisplay};
use rl_sys::history::{listmgmt, mgmt};
use tungstenite::Message;
use url::Url;

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

fn wscat_client(url: Url, _auth_option: Option<String>) -> Result<()> {
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

fn wscat_server(port: usize) -> Result<()> {
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

fn main() -> Result<()> {
    // Command line interface
    let matches = App::new("manx")
        .version(crate_version!())
        .author("Walther Chen <walther.chen@gmail.com>")
        .about("Talk to websockets from cli")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(SubCommand::with_name("connect")
            .visible_alias("c")
            . about("Connect to server url")
            . arg(Arg::with_name("URL")
                .index(1)
                .required(true))
            .arg(Arg::with_name("USERNAME:PASSWORD")
                .long("auth")
                .help("Add basic HTTP authentication header. (connect only)")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("listen")
            .about("Listen on port")
            .arg(Arg::with_name("PORT")
                .index(1)
                .required(true)))
        .get_matches();

    // Startup client or server
    if let Some(ref matches) = matches.subcommand_matches("connect") {
        if let Some(url_option) = matches.value_of("URL") {
            let url: Url = url_option.parse()
                .with_context(|| format!("Error parsing {:?}", url_option))?;

            // TODO later
            //let auth_option = matches.value_of("USERNAME:PASSWORD")
            //    .and_then(|user_pass| {
            //        parse_authorization(user_pass)
            //    });
            let auth_option = None;

            // print that client is connecting
            let out_url = format!("Connected to {:?} (Ctrl-C to exit)", url_option);
            println!("{}", Blue.bold().paint(out_url));
            wscat_client(url, auth_option)?;
        }

    } else if let Some(ref matches) = matches.subcommand_matches("listen") {
        if let Some(port_option) = matches.value_of("PORT") {
            let port: usize = match port_option.parse() {
                Ok(port) if port <= 65535 => port,
                Ok(port) => {
                    let out = format!("Port '{:?}' not in range", port);
                    println!("{}", Red.paint(out));
                    process::exit(1);
                },
                Err(err) => {
                    let out = format!("Error parsing {:?} ({:?})", port_option, err);
                    println!("{}", Red.paint(out));
                    process::exit(1);
                },
            };
            wscat_server(port)?;
        }
    }

    Ok(())
}

