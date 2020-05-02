mod cli;
mod client;
mod server;

use std::process;

use anyhow::{Context as _, Result};
use ansi_term::Colour::{Blue, Red};
use url::Url;

fn main() -> Result<()> {
    // Command line interface
    let matches = cli::get_cli();

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
            client::wscat_client(url, auth_option)?;
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
            server::wscat_server(port)?;
        }
    }

    Ok(())
}

