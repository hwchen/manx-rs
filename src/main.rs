// prompt still disappears every now and then
#[macro_use]
extern crate ansi_term;
extern crate clap;
extern crate rl_sys;
extern crate url;
extern crate websocket;

use std::error::Error;
use std::io::{self, Write};
use std::process;
use std::str;
use std::sync::mpsc::channel;
use std::thread;

use ansi_term::Colour::{Blue, Green, Red, White};
use clap::{App, Arg, SubCommand};
use rl_sys::readline;
use rl_sys::readline::redisplay;
use rl_sys::history::{listmgmt, mgmt};
use url::ParseError;
use websocket::{Client, Message, Sender, Receiver, Server};
use websocket::client::request::Url;
use websocket::message::Type;
use websocket::result::WebSocketError::{WebSocketUrlError, IoError};
use websocket::result::WSUrlErrorKind::InvalidScheme;

fn wscat_client(url: Url) {
    let request = match Client::connect(url) {
        Ok(r) => r,
        Err(WebSocketUrlError(InvalidScheme)) => {
            let out = format!("Invalid Scheme, url must start with 'ws://' or 'wss://'");
            println!("{}", Red.paint(out));
            process::exit(1);
        },
        Err(IoError(err)) => {
            // check back later... why does this description()
            // return "connection refused", when
            // code for WebSocketError seems to return "I/O failure"
            let out = format!("Error: {}", err.description());
            println!("{}", Red.paint(out));
            process::exit(1);
        },
        Err(err) => {
            let out = format!("Error connecting: {:?}", err);
            println!("{}", Red.paint(out));
            process::exit(1);
        }
    };
    let response = request.send().unwrap();
    //response.validate().expect("request refused");
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
            let out_ip = format!("Connection from {}", ip);
            println!("{}", Green.paint(out_ip));

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
                    _ => {
                        println!("<< {} {}",
                            str::from_utf8(&message.payload).unwrap(),
                            White.dimmed().paint(format!("({})", ip))
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
}

fn main() {
    // Command line interface
    let matches = App::new("wscat-rs")
        .version("0.1")
        .author("Walther Chen <walther.chen@gmail.com>")
        .about("Talk to websockets from cli")
        .subcommand(SubCommand::with_name("connect")
             .about("Connect to server url")
             .arg(Arg::with_name("URL")
                .index(1)
                .required(true))
            .arg(Arg::with_name("HEADER:VALUE")
                .short("H")
                .long("header")
                .help("set HTTP header, repeat to set multiple. (connect only)")
                .takes_value(true))
            .arg(Arg::with_name("USERNAME:PASSWORD")
                .long("auth")
                .help("Add basic HTTP authentication header. (connect only)")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("listen")
             .about("Listen on port")
             .arg(Arg::with_name("PORT")
                .index(1)
                .required(true)))
        .arg(Arg::with_name("PROTOCOL")
             .short("p")
             .long("protocol")
             .help("optional protocol version")
             .takes_value(true))
        .arg(Arg::with_name("ORIGIN")
             .short("o")
             .long("origin")
             .help("optional origin")
             .takes_value(true))
        .arg(Arg::with_name("HOST")
             .long("host")
             .help("optional host")
             .takes_value(true))
        .arg(Arg::with_name("SUBPROTOCOL")
             .short("s")
             .long("subprotocol")
             .help("optional subprotocol")
             .takes_value(true))
        .arg(Arg::with_name("NO_CHECK")
             .short("n")
             .long("no-check")
             .help("Do not check for unauthorized certificates")
             .takes_value(false))
        .get_matches();

    // Options processing here (let some...)


    // Startup client or server
    if let Some(ref matches) = matches.subcommand_matches("connect") {
        if let Some(url_option) = matches.value_of("URL") {
            let url: Url = match url_option.parse() {
                Ok(url) => url,
                Err(ParseError::RelativeUrlWithoutBase) => {
                    let out = format!("Error parsing {:?}, url must begin with base", url_option);
                    println!("{}", Red.paint(out));
                    process::exit(1);
                }
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
            wscat_server(port);
        }
    }
}

