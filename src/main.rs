// prompt still disappears every now and then
#[macro_use]
extern crate clap;
extern crate rl_sys;
extern crate ansi_term;
extern crate websocket;

use std::io::{self, Write};
use std::process;
use std::str;
use std::sync::mpsc::channel;
use std::thread;

use clap::{App, Arg};
use rl_sys::readline;
use rl_sys::readline::redisplay;
use rl_sys::history::{listmgmt, mgmt};
use ansi_term::Colour::{Blue, Green, Red, White};
use websocket::{Client, Message, Sender, Receiver, Server};
use websocket::client::request::Url;
use websocket::message::Type;
use websocket::result::WebSocketError::WebSocketUrlError;
use websocket::result::WSUrlErrorKind::InvalidScheme;

fn wscat_client(url: Url) {
    let request = match Client::connect(url) {
        Ok(r) => r,
        Err(WebSocketUrlError(InvalidScheme)) => {
            let out = format!("Invalid Scheme, url must start with 'ws://' or 'wss://'");
            println!("{}", Red.paint(out));
            process::exit(1);
        },
        Err(err) => {
            let out = format!("Error connecting:{:?}", err);
            println!("{}", Red.paint(out));
            process::exit(1);
        }
    };
    let response = request.send().unwrap();
    response.validate().unwrap();

    let client = response.begin();
    let (mut sender, mut receiver) = client.split();

    // Move from here onwards to its own function, so can have a listening fn also.
    // channel for sending messages from readline to ws send thread
    let (tx, rx) = channel();

    // Thread for sending to ws
    let send = thread::spawn( move || {
        loop {
            let message: Message = rx.recv().unwrap();
            sender.send_message(&message).expect("err sending message");
        }
    });

    // Thread for receiving from ws
    let tx_1 = tx.clone();
    let receive = thread::spawn( move || {
        for message in receiver.incoming_messages() {
            let message: Message = match message {
                Ok(m) => m,
                _ => break // Handle this?
            };

            //write to stdout depending on opcode
            let out = match message.opcode {
                Type::Ping => {
                    tx_1.send(Message::pong(message.payload)).unwrap();
                    format!("{}", Green.paint("Ping!\n")) //add color
                },
                Type::Text => {
                    let out = format!("<< {}\n", str::from_utf8(&message.payload).unwrap());
                    format!("{}", White.dimmed().paint(out))
                },
                _ => format!("Other type of ws message"),
            };

            redisplay::save_prompt();

            //clear line, maybe there's easier way in readline
            let esc = String::from_utf8(vec![27]).unwrap();
            let clear_line_bytes = format!("{}[2K", esc).into_bytes();
            io::stdout().write(&clear_line_bytes[..]).expect("error clearing line");
            io::stdout().flush().unwrap();

            redisplay::message(&out).unwrap();
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
    }

    mgmt::cleanup();

    // unwrap which exits program
    send.join().unwrap();
    receive.join().unwrap();
}

fn wscat_server(port: usize) {
    let out_port = format!("Listening on port {:?}", port);
    println!("{}", Blue.bold().paint(out_port));
    let url = format!("127.0.0.1:{}", port); 
    let server = match Server::bind(&url[..]) {
        Ok(c) => c,
        Err(err) => {
            let out = format!("Error connecting:{:?}", err);
            println!("{}", Red.paint(out));
            process::exit(1);
        }
    };
    let mut handles = Vec::new();
    for connection in server {
        let handle = thread::spawn(move || {
            let request = connection.unwrap().read_request().unwrap();
            request.validate().unwrap();

            let response = request.accept();
            let mut client = response.send().unwrap();

            let ip = client.get_mut_sender()
                .get_mut()
                .peer_addr()
                .unwrap();
            println!("Connection from {}", ip);

            let (mut sender, mut receiver) = client.split();

            for message in receiver.incoming_messages() {
                let message: Message = match message {
                    Ok(m) => m,
                    Err(_) => {
                        let out = format!("Disconnecting {}", ip);
                        println!("{}", Red.paint(out));
                        break;
                    }
                };

                match message.opcode {
                    Type::Close => {
                        let message = Message::close();
                        sender.send_message(&message).unwrap();
                        println!("Client {} disconnected", ip);
                        return;
                    },
                    Type::Ping => {
                        let message = Message::pong(message.payload);
                        sender.send_message(&message).unwrap();
                    },
                    _ => println!("<< {}", str::from_utf8(&message.payload).unwrap())
                }
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }
}

fn main() {
    // Command line interface
    let matches = App::new("wscat-rs")
        .version("0.1")
        .author("Walther Chen <walther.chen@gmail.com>")
        .about("Talk to websockets from cli")
        .arg(Arg::with_name("CONNECT")
             .short("c")
             .long("connect")
             .help("Connect to server url")
             .takes_value(true))
        .arg(Arg::with_name("LISTEN")
             .short("l")
             .long("listen")
             .help("Listen on port")
             .takes_value(true))
        .get_matches();

    // check that there isn't both a connect and a listen
    // Early exit if both exist
    if let Some(_) = matches.value_of("CONNECT") {
        if let Some(_) = matches.value_of("LISTEN") {
            println!("{}",
                Red.paint("Cannot have both 'Connect' and 'Listen' options simultaneously")
            );
            process::exit(1);
        }
    }

    // Options processing here (let some...)

    // Startup client or server
    if let Some(url_option) = matches.value_of("CONNECT") {
        let url: Url = match url_option.parse() {
            Ok(url) => url,
            Err(err) => {
                let out = format!("Error parsing {:?} ({:?})", url_option, err);
                println!("{}", Red.paint(out));
                process::exit(1);
            }
        };

        // print that client is connecting
        let out_url = format!("Connecting to {:?}", url_option);
        println!("{}", Blue.bold().paint(out_url));

        wscat_client(url);

    } else if let Some(port_option) = matches.value_of("LISTEN") {
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
        wscat_server(port);
    }
}

