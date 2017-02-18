# Manx

manx is a wscat clone. It's a simple interface to websockets. Intended to be used as
a client, but also contains a listener.

One of its features is that it saves the prompt, so that you can type commands even
if you are receiving a flood of data.

# Install

Make sure you have [Rust](https://rustup.rs), then

```
$ cargo install manx
```

## Usage

```
manx 0.3.0
Walther Chen <walther.chen@gmail.com>
Talk to websockets from cli

USAGE:
    manx [FLAGS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    connect    Connect to server url [aliases: c]
    help       Prints this message or the help of the given subcommand(s)
    listen     Listen on port
```

## Version 1.0 plans

I'm considering rewriting to bring it to 1.0. Changes include:

- new architecture, based on ripgrep cli app.
- with new architecture, handle errors with error-chain.
- rustyline instead of readline.
- ? better colors handling.
- ? async websockets.

But, I might just keep it as-is, it seems to work even if the code
is not the prettiest.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

