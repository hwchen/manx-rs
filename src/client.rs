use ansi_term::Colour::{Green, Red};
use anyhow::{bail, Context as _, Result};
use async_channel::{bounded as chan, Receiver, Sender};
use async_executor::{LocalExecutor, Task};
use async_tungstenite::tungstenite::Message;
use futures_util::stream::StreamExt;
use futures_lite::future::{self, block_on};
use linefeed::{ReadResult, Signal};
use std::process;
use std::sync::Arc;
use std::thread;
use url::Url;

use crate::ws;

// Three threads:
// - stdin loop
// - stdout loop
// - websocket async (read and write tasks spawned)
//
// Use channels to communicate across threads.
// - block_on for async hannel when receiver is in sync stdout
// - piper when receiver is in websocket async
//
pub fn wscat_client(url: Url, opts: Opts) -> Result<()> {
    // set up channels for communicating
    let (tx_to_stdout, mut rx_stdout) = chan::<String>(10); // async -> sync
    let (tx_to_ws_write, rx_ws_write) = chan::<Message>(10); // sync -> async, async -> async

    let chans = Channels {
        tx_to_ws_write: tx_to_ws_write.clone(),
        tx_to_stdout,
        rx_ws_write,
    };

    // run read/write tasks for websocket
    let ws_handle = thread::spawn(|| {
        let local_ex = LocalExecutor::new();
        local_ex.run(async {
            if let Err(err) = watch_ws(url, chans, opts).await {
                let out = format!("{:#}", err);
                eprintln!("\n{}", Red.paint(out));
                process::exit(0);
            }
        })
    });

    // readline interface, which will hold read/write locks
    let readline = linefeed::Interface::new("manx")?;
    readline.set_prompt("> ")?;
    readline.set_report_signal(Signal::Interrupt, true);
    let readline = Arc::new(readline);

    //stdout loop
    let stdout_readline = readline.clone();
    let stdout_handle = thread::spawn(move || {
        loop {
            if let Some(message) = block_on(rx_stdout.next()) {
                let mut w = stdout_readline.lock_writer_erase().unwrap();
                writeln!(w, "<< {}", message).unwrap();
            }
        }
    });

    // stdin loop
    loop {
        match readline.read_line()? {
            ReadResult::Input(input) => {
                readline.add_history(input.clone());
                // must block on this channel
                block_on(tx_to_ws_write.send(Message::text(input)))?;
            },
            ReadResult::Signal(sig) => {
                // If I don't exit process here, readline loop exits on first Interrupt, and then
                // the rest of the program exists on the second Interrupt
                if sig == Signal::Interrupt {
                    readline.cancel_read_line()?;
                    process::exit(0)
                };
            },
            _ => break,
        }
    }

    ws_handle.join().unwrap();
    stdout_handle.join().unwrap();

    Ok(())
}

// all spawns (Task::local) are in the context of a LocalExecutor, since this fn is run in that
// context.
async fn watch_ws(url: Url, chans: Channels, opts: Opts) -> Result<()> {
    let show_ping_pong = opts.show_ping_pong;

    let Channels {tx_to_ws_write, tx_to_stdout, rx_ws_write } = chans;
    let tx_to_ws_write = tx_to_ws_write.clone();

    let stream = ws::init(url, opts.cert).await?;
    let (writer, mut reader) = stream.split();

    // read task reads from ws, then sends signal to stdout loop
    let read_task = Task::local(async move {
        while let Some(message) = reader.next().await {
            let message = message.context("Connection closed")?;

            // If prepare a message for display in stdout.
            let out = match message {
                Message::Ping(payload) => {
                    tx_to_ws_write.send(Message::Pong(payload)).await?;
                    if show_ping_pong {
                        format!("{}", Green.paint("-- received ping"))
                    } else {
                        continue;
                    }
                },
                Message::Pong(_) => {
                    if show_ping_pong {
                        format!("{}", Green.paint("-- received pong"))
                    } else {
                        continue
                    }
                },
                Message::Text(payload) => { payload },
                Message::Binary(payload) => {
                    // Binary just supported as Utf8 text here; no downloading, etc.
                    // TODO figure out a better way to support?
                    String::from_utf8(payload)?
                },
                Message::Close(_) => {
                    bail!("Close message received"); // not really an error
                },
            };

            // blocking
            tx_to_stdout.send(out).await?;
        }

        Ok(())
    });

    let write_task = Task::local(async {
        rx_ws_write.map(Ok).forward(writer).await?;
        Ok(())
    });

    future::try_join(read_task, write_task).await?;

    Ok(())
}

struct Channels {
    tx_to_ws_write: Sender<Message>,
    tx_to_stdout: Sender<String>,
    rx_ws_write: Receiver<Message>,
}

pub struct Opts {
    pub auth: Option<String>,
    pub show_ping_pong: bool,
    pub cert: Option<Vec<u8>>,
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
