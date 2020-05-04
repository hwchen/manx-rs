use ansi_term::Colour::{Green, Red};
use anyhow::{bail, Context as _, Result};
use async_tungstenite::tungstenite::Message;
use futures::future;
use futures::stream::StreamExt;
use linefeed::{ReadResult, Signal};
use smol::{Async, Task};
use std::net::TcpStream;
use std::process;
use std::sync::Arc;
use std::thread;
use url::Url;

// Three threads:
// - stdin loop
// - stdout loop
// - websocket async (read and write tasks spawned)
//
// Use channels to communicate across threads.
// - blocking channel when receiver is in sync stdout?
// - piper when receiver is in websocket async
//
// First just support ws, not wss
pub fn wscat_client(url: Url, _auth_option: Option<String>) -> Result<()> {
    // set up channels for communicating
    let (tx_to_stdout, mut rx_stdout) = piper::chan::<String>(10); // async -> sync
    let (tx_to_ws_write, rx_ws_write) = piper::chan::<Message>(10); // sync -> async, async -> async

    let chans = WsChannels {
        tx_to_ws_write: tx_to_ws_write.clone(),
        tx_to_stdout,
        rx_ws_write,
    };

    // run read/write tasks for websocket
    let ws_handle = thread::spawn(|| {
        smol::run(async {
            if let Err(err) = do_ws(url, chans).await {
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
            if let Some(message) = smol::block_on(rx_stdout.next()) {
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
                smol::block_on(tx_to_ws_write.send(Message::text(input)));
            },
            ReadResult::Signal(sig) => {
                // If I don't exit process here, readline loop exits on first Interrupt, and then
                // the rest of the program exists on the second Interrupt
                if sig == Signal::Interrupt { process::exit(0) };
            },
            _ => break,
        }
    }

    ws_handle.join().unwrap();
    stdout_handle.join().unwrap();

    Ok(())
}

// Only use thread-local executor, since smol will only run on one thread.
async fn do_ws(url: Url, chans: WsChannels) -> Result<()> {
    let WsChannels {tx_to_ws_write, tx_to_stdout, rx_ws_write } = chans;
    let tx_to_ws_write = tx_to_ws_write.clone();

    let host = url.host_str().context("Can't parse host")?;
    let port = url.port_or_known_default().context("Can't guess port")?;
    let addr = format!("{}:{}", host, port);

    let stream = Async::<TcpStream>::connect(&addr).await?;
    let (stream, _resp) = async_tungstenite::client_async(&url, stream).await?;

    let (writer, mut reader) = stream.split();

    // read task reads from ws, then sends signal to stdout loop
    let read_task = Task::local(async move {
        while let Some(message) = reader.next().await {
            let message = message.context("Connection closed")?;

            // If prepare a message for display in stdout.
            let out = match message {
                Message::Ping(payload) => {
                    tx_to_ws_write.send(Message::Pong(payload)).await;
                    format!("{}", Green.paint("Ping!\n"))
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
                _ => format!("Unsupported ws message"),
            };

            // blocking
            tx_to_stdout.send(out).await;
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

struct WsChannels {
    tx_to_ws_write: piper::Sender<Message>,
    tx_to_stdout: piper::Sender<String>,
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
