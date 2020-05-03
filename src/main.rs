mod cli;
mod client;

use anyhow::{Context as _, Result};
use ansi_term::Colour::Blue;
use url::Url;

fn main() -> Result<()> {
    // Command line interface
    let matches = cli::get_cli();

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
        let out_url = format!("Connecting to {:?} (Ctrl-C to exit)", url_option);
        println!("{}", Blue.bold().paint(out_url));
        client::wscat_client(url, auth_option)?;
    }

    Ok(())
}

