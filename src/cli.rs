use clap::{crate_version, App, AppSettings, Arg, ArgMatches};

pub fn get_cli<'a>() -> ArgMatches<'a> {
    // Command line interface
    App::new("manx")
        .version(crate_version!())
        .author("Walther Chen <walther.chen@gmail.com>")
        .about("Talk to websockets from cli")
        .setting(AppSettings::ArgRequiredElseHelp)
        . arg(Arg::with_name("URL")
            .index(1)
            .required(true))
        //.arg(Arg::with_name("auth")
        //    .long("auth")
        //    .value_name("USER:PASSWORD")
        //    .help("Add basic HTTP authentication header.")
        //    .takes_value(true))
        .arg(Arg::with_name("show_ping_pong")
            .long("show-ping-pong")
            .help("Print when ping or pong received.")
            .takes_value(false))
        .get_matches()
}
