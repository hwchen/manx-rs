use ansi_term::Colour::{Green, Red, White};
use anyhow::Result;
use async_tungstenite::tungstenite::Message;
use futures::{future, pin_mut};
use futures::stream::StreamExt;
use rl_sys::readline::{self, redisplay};
use rl_sys::history::{listmgmt, mgmt};
use smol::{Async, Task};
use std::io::{self, Write};
use std::net::TcpStream;
use std::process;
use std::sync::mpsc::channel;
use std::thread;
use url::Url;

// Three threads:
// - stdin loop
// - stdout loop
// - websocket async (read and write tasks spawned)
//
// Use channels to communicate across threads.
// - Crossbeam channel when receiver is in sync stdout
// - piper when receiver is in websocket async
//
// First just support ws, not wss
pub fn wscat_client(url: Url, _auth_option: Option<String>) -> Result<()> {
    // set up channels for communicating
    let (tx_to_stdout, rx_stdout) = channel::<Message>(); // async -> sync
    let (tx_to_ws_write, rx_ws_write) = piper::chan::<Message>(10); // sync -> async, async -> async

    let chans = WsChannels {
        tx_to_ws_write: tx_to_ws_write.clone(),
        tx_to_stdout,
        rx_ws_write,
    };

    // run read/write tasks for websocket
    let ws_handle = thread::spawn(|| smol::run(ws_client(url, chans)));

    //stdout loop
    let stdout_handle = thread::spawn(|| {
        for message in rx_stdout {
            if !(message.is_text() || message.is_binary()) {
                continue;
            }

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

            io::stdout().write(&message.into_text().unwrap().as_bytes()).unwrap();
            io::stdout().flush().unwrap();
            redisplay::on_new_line().unwrap();
            redisplay::rl_restore_prompt();
            redisplay::redisplay();
        }
    });

    // stdin loop
    loop {
        let input = match readline::readline("> ") {
            Ok(Some(i)) => i,
            Ok(None) => continue,
            _ => break,
        };
        listmgmt::add(&input).unwrap();
        // block on this
        let _ = smol::block_on(async {tx_to_ws_write.send(Message::text(input)).await});
    }
    mgmt::cleanup();

    ws_handle.join().unwrap().unwrap();
    stdout_handle.join().unwrap();

    Ok(())
}

// only use thread-local executor, since smol will only run on one thread
async fn ws_client(addr: Url, chans: WsChannels) -> Result<()> {
    let WsChannels {tx_to_ws_write, tx_to_stdout, rx_ws_write } = chans;
    let tx_to_ws_write = tx_to_ws_write.clone();

    let stream = Async::<TcpStream>::connect("127.0.0.1:9999").await?;
    let (stream, _resp) = async_tungstenite::client_async(&addr, stream).await?;

    let (writer, mut reader) = stream.split();

    // read task reads from ws, then sends signal to stdout loop
    let read_task = Task::local(async move {
        while let Some(message) = reader.next().await {
            let message: Message = match message {
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
                    tx_to_ws_write.send(Message::Pong(payload)).await;
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

            // blocking
            // TODO try crossbeam channel?
            tx_to_stdout.send(Message::text(out)).unwrap();
        }
    });

    // TODO remove this unwrap
    let write_task = Task::local(async {
        rx_ws_write.map(Ok).forward(writer).await
    });

    pin_mut!(read_task, write_task);
    future::select(read_task, write_task).await;

    Ok(())
}

struct WsChannels {
    tx_to_ws_write: piper::Sender<Message>,
    tx_to_stdout: std::sync::mpsc::Sender<Message>,
    rx_ws_write: piper::Receiver<Message>,
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
