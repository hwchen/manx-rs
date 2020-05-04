# Manx

manx is a wscat clone. It's a simple interface to websocket servers.

One of its features is that it saves the prompt, so that you can type commands even
if you are receiving a flood of data.

For those learning Rust and async, manx shows how async websockets can interface with sync stdin and stdout loops.

Windows should be supported (although currently untested, please sent in bug reports!)

Thanks especially to [smol](https://github.com/stjepang/smol), [async-tungstenite](https://github.com/sdroege/async-tungstenite), [tungstenite](https://github.com/snapview/tungstenite-rs), and [linefeed](https://github.com/murarth/linefeed), which provided the building blocks for this app.

# Install

Make sure you have [Rust](https://rustup.rs), then

```
$ cargo install manx
```

## Usage

```
manx 0.4.0
Walther Chen <walther.chen@gmail.com>
Talk to websockets from cli

USAGE:
    manx [FLAGS] [OPTIONS] <URL>

FLAGS:
    -h, --help              Prints help information
        --show-ping-pong    Print when ping or pong received.
    -V, --version           Prints version information

OPTIONS:
        --cert <cert_path>    Specify a client SSL Certificate

ARGS:
    <URL>
```

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

