// todo: update to better readline option (linenoise or rustyline)
// linenoise for some reason doesn't take ctrl-c
// Also, what happens when message is received when input prompt is
// already up?
extern crate clap;
extern crate readline;
extern crate websocket;

use std::io::{self, Write};
use std::str;
use std::thread;

use websocket::{Client, Message, Sender, Receiver};
use websocket::client::request::Url;
use websocket::message::Type;

fn main() {
    let url = Url::parse("ws://echo.websocket.org").unwrap();
    let request = Client::connect(url).unwrap();
    let response = request.send().unwrap();
    response.validate().unwrap();

    let client = response.begin();
    let (mut sender, mut receiver) = client.split();

    let i = thread::spawn( move || {
        loop {
            let input = readline::readline("> ").unwrap();
            readline::add_history(&input);
            sender.send_message(&Message::text(input)).unwrap();
        }
    });

    let o = thread::spawn( move || {
        for message in receiver.incoming_messages() {
            let message: Message = message.unwrap();
            io::stdout().write(b"\033[2K\033[E").unwrap();
            io::stdout().flush().unwrap();
            match message {
                Message{opcode, payload, ..} => {
                    match opcode {
                        Type::Ping => {
                            println!("Ping!"); //add color
                            //sender.send_message(&Message::pong(payload)).unwrap();
                        },
                        Type::Text => {
                            println!("\n<< {}", str::from_utf8(&payload.into_owned()[..]).unwrap());
                        },
                        _ => println!("Some other type of ws message"),
                    }
                }
            }
        }
    });

    i.join().unwrap();
    o.join().unwrap();

}
