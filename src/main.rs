mod cli;
mod client;
mod ws;

use anyhow::{Context as _, Result};
use ansi_term::Colour::Blue;
use url::Url;

fn main() -> Result<()> {
    // Command line interface
    let matches = cli::get_cli();

    let cert= matches.value_of("cert_path")
        .map(|path| {
            use std::io::Read;
            let mut res = Vec::new();
            let mut f = std::fs::File::open(path)?;
            f.read_to_end(&mut res)?;

            Ok::<_, std::io::Error>(res)
        })
        .transpose()
        .context("Could not read certificate")?;

    let opts = client::Opts {
        auth: matches.value_of("auth").map(|s| s.to_owned()),
        show_ping_pong: matches.is_present("show_ping_pong"),
        cert,
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

