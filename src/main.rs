mod cli;
mod client;

use anyhow::{Context as _, Result};
use ansi_term::Colour::Blue;
use url::Url;

fn main() -> Result<()> {
    // Command line interface
    let matches = cli::get_cli();

    let opts = client::Opts {
        auth: matches.value_of("auth").map(|s| s.to_owned()),
        show_ping_pong: matches.is_present("show_ping_pong"),
    };

    if let Some(url_option) = matches.value_of("URL") {
        let url: Url = url_option.parse()
            .with_context(|| format!("Error parsing {:?}", url_option))?;

        // print that client is connecting
        let out_url = format!("Connecting to {:?} (Ctrl-C to exit)", url_option);
        println!("{}", Blue.bold().paint(out_url));
        client::wscat_client(url, opts)?;
    }

    Ok(())
}

