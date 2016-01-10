// prompt still disappears every now and then
#[macro_use]
extern crate clap;
extern crate rl_sys;
extern crate ansi_term;
extern crate websocket;

use std::io::{self, Write};
use std::str;
use std::sync::mpsc::channel;
use std::thread;

use clap::App;
use rl_sys::readline;
use rl_sys::readline::redisplay;
use rl_sys::history::{listmgmt, mgmt};
use ansi_term::Colour::{Green, Blue, White};
use websocket::{Client, Message, Sender, Receiver};
use websocket::client::request::Url;
use websocket::message::Type;

fn main() {
    // Command line interface
    let matches = App::new("wscat-rs")
        .version("0.1")
        .author("Walther Chen <walther.chen@gmail.com>")
        .about("Talk to websockets from cli")
        .args_from_usage(
            "-c --connect=[CONNECT] 'Server url to connect to'")
        .get_matches();


    // channel for sending messages from readline to ws send thread
    let (tx, rx) = channel();

    // set up client for ws
    let url_option = matches.value_of("CONNECT").unwrap_or("ws://echo.websocket.org");
    let out_url = format!("Connecting to {:?}", url_option);
    println!("{}", Blue.bold().paint(out_url));
    let url = Url::parse(url_option).unwrap();
    let request = Client::connect(url).unwrap();
    let response = request.send().unwrap();
    response.validate().unwrap();

    let client = response.begin();
    let (mut sender, mut receiver) = client.split();

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

