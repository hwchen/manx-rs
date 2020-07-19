// from smol/websocket-client example.

use anyhow::{bail, Context as _, Result};
use async_native_tls::{Certificate, TlsConnector, TlsStream};
use async_tungstenite::{tungstenite::{self, Message}, WebSocketStream};
use blocking::unblock;
use futures::prelude::*;
use async_io::Async;
use std::net::{TcpStream, ToSocketAddrs};
use std::pin::Pin;
use std::task::{Context, Poll};
use url::Url;

pub enum WsStream {
    Plain(WebSocketStream<Async<TcpStream>>),
    Tls(WebSocketStream<TlsStream<Async<TcpStream>>>),
}

impl Sink<Message> for WsStream {
    type Error = tungstenite::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut *self {
            WsStream::Plain(s) => Pin::new(s).poll_ready(cx),
            WsStream::Tls(s) => Pin::new(s).poll_ready(cx),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        match &mut *self {
            WsStream::Plain(s) => Pin::new(s).start_send(item),
            WsStream::Tls(s) => Pin::new(s).start_send(item),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut *self {
            WsStream::Plain(s) => Pin::new(s).poll_flush(cx),
            WsStream::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut *self {
            WsStream::Plain(s) => Pin::new(s).poll_close(cx),
            WsStream::Tls(s) => Pin::new(s).poll_close(cx),
        }
    }
}

impl Stream for WsStream {
    type Item = tungstenite::Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut *self {
            WsStream::Plain(s) => Pin::new(s).poll_next(cx),
            WsStream::Tls(s) => Pin::new(s).poll_next(cx),
        }
    }
}

pub async fn init(url: Url, cert: Option<Vec<u8>>) -> Result<WsStream> {
    let host = url.host_str().context("Can't parse host")?.to_owned();
    let port = url.port_or_known_default().context("Can't guess port")?;

    let socket_addr = {
        let host = host.clone();
        unblock!(
            (host.as_str(), port).to_socket_addrs()
        )?
        .next()
        .context("cannot resolve address")?
    };

    let res = match url.scheme() {
        "ws" => {
            let stream = Async::<TcpStream>::connect(socket_addr).await?;
            let (stream, _resp) = async_tungstenite::client_async(&url, stream).await?;
            WsStream::Plain(stream)
        },
        "wss" => {
            // init tls
            let cert = cert.context("No certificate found for tls")?;
            let mut tls_builder = native_tls::TlsConnector::builder();
            tls_builder.add_root_certificate(Certificate::from_pem(&cert)?);
            let tls = TlsConnector::from(tls_builder);

            //
            let stream = Async::<TcpStream>::connect(socket_addr).await?;
            let stream = tls.connect(host, stream).await?;
            let (stream, _resp) = async_tungstenite::client_async(&url, stream).await?;
            WsStream::Tls(stream)
        },
        _ => bail!("unsupported scheme")
    };

    Ok(res)
}
