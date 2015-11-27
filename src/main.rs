// todo: update to better readline option (linenoise or rustyline)
// linenoise for some reason doesn't take ctrl-c
//
// Must do now:
// Also, need to be able to close both threads gracefully
//
// Then colors

extern crate clap;
extern crate readline;
extern crate websocket;

use std::io::{self, Write};
use std::str;
use std::sync::mpsc;
use std::thread;

use websocket::{Client, Message, Sender, Receiver};
use websocket::client::request::Url;
use websocket::message::Type;

fn main() {
    // Move this out of main
    // Clear line in cli
    let esc = String::from_utf8(vec![27]).unwrap();
    let clear_line = format!("{}[2K{}[E", esc, esc);
    let clear_line_bytes = clear_line.into_bytes();

    //set up channel for syncing
    
    let (tx, rx) = mpsc::channel();

    // set up client for ws
    let url = Url::parse("ws://echo.websocket.org").unwrap();
    let request = Client::connect(url).unwrap();
    let response = request.send().unwrap();
    response.validate().unwrap();

    let client = response.begin();
    let (mut sender, mut receiver) = client.split();

    // Thread for sending to ws
    let tx = tx.clone();
    let i = thread::spawn( move || {
        tx.send(42).unwrap();
        loop {
//            if let Ok(received) = rx.recv() {
//                match received {
//                    "broken pipe" => break,
//                    _ => continue,
//                }
//            }
            let input = readline::readline("> ").expect("no input at prompt");
            readline::add_history(&input);
            sender.send_message(&Message::text(input)).expect("problem sending message");
        }
    });

    // Thread for receiving from ws
    let tx = tx.clone();
    let o = thread::spawn( move || {
        tx.send(42).unwrap();
        for message in receiver.incoming_messages() {
            let message: Message = match message {
                Ok(message) => message,
                Err(_) => {
                    println!("Broken pipe");
                    break
                }
            };
            io::stdout().write(&clear_line_bytes[..]).expect("error clearing line");
            match message {
                Message{opcode, payload, ..} => {
                    match opcode {
                        Type::Ping => {
                            println!("Ping!"); //add color
                            //sender.send_message(&Message::pong(payload)).unwrap();
                        },
                        Type::Text => {
                            let out = format!("<< {}\n> ",
                                str::from_utf8(&payload.into_owned()[..])
                                .unwrap()
                            );
                            let out_bytes = out.into_bytes();
                            io::stdout().write(&out_bytes[..]).expect("error on write");
                            io::stdout().flush().expect("error on flush");
                        },
                        _ => println!("Some other type of ws message"),
                    }
                }
            }
        }
        println!("Exited loop for receiving");
    });

    // unwrap to error which exits program
    i.join().unwrap();
    o.join().unwrap();
    rx.recv().expect("from channel error");

}
