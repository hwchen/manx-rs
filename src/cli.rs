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
        .arg(Arg::with_name("USERNAME:PASSWORD")
            .long("auth")
            .help("Add basic HTTP authentication header. (connect only)")
            .takes_value(true))
        .get_matches()
}
