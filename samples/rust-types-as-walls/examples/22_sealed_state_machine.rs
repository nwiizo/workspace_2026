//! sealed trait + PhantomData で状態遷移を閉じた state machine として表現する。
//! 「ありうる状態」と「許された遷移」の両方を型で固定できる。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use std::marker::PhantomData;
use thiserror::Error;

mod sealed {
    pub trait Sealed {}
}

trait ConnectionState: sealed::Sealed {
    fn label() -> &'static str;
}

#[derive(Debug, Clone, Copy)]
struct Closed;

#[derive(Debug, Clone, Copy)]
struct Listening;

#[derive(Debug, Clone, Copy)]
struct Established;

impl sealed::Sealed for Closed {}
impl sealed::Sealed for Listening {}
impl sealed::Sealed for Established {}

impl ConnectionState for Closed {
    fn label() -> &'static str {
        "closed"
    }
}

impl ConnectionState for Listening {
    fn label() -> &'static str {
        "listening"
    }
}

impl ConnectionState for Established {
    fn label() -> &'static str {
        "established"
    }
}

#[derive(Debug)]
struct TcpConnection<State: ConnectionState> {
    local_addr: String,
    peer_addr: Option<String>,
    _state: PhantomData<State>,
}

#[derive(Debug, Error)]
enum ConnectionError {
    #[error("ローカルアドレスが空です")]
    EmptyLocalAddr,
    #[error("接続先アドレスが空です")]
    EmptyPeerAddr,
}

impl TcpConnection<Closed> {
    fn new(local_addr: impl Into<String>) -> Result<Self, ConnectionError> {
        let local_address = local_addr.into();
        if local_address.trim().is_empty() {
            return Err(ConnectionError::EmptyLocalAddr);
        }

        Ok(Self {
            local_addr: local_address,
            peer_addr: None,
            _state: PhantomData,
        })
    }

    fn listen(self) -> TcpConnection<Listening> {
        TcpConnection {
            local_addr: self.local_addr,
            peer_addr: None,
            _state: PhantomData,
        }
    }
}

impl TcpConnection<Listening> {
    fn accept(
        self,
        peer_addr: impl Into<String>,
    ) -> Result<TcpConnection<Established>, ConnectionError> {
        let peer_address = peer_addr.into();
        if peer_address.trim().is_empty() {
            return Err(ConnectionError::EmptyPeerAddr);
        }

        Ok(TcpConnection {
            local_addr: self.local_addr,
            peer_addr: Some(peer_address),
            _state: PhantomData,
        })
    }
}

impl TcpConnection<Established> {
    fn send(&self, payload: &[u8]) {
        println!(
            "{} -> {:?}: {} bytes",
            self.local_addr,
            self.peer_addr,
            payload.len()
        );
    }
}

impl<State: ConnectionState> TcpConnection<State> {
    fn state_label(&self) -> &'static str {
        State::label()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = TcpConnection::<Closed>::new("127.0.0.1:8080")?;
    println!("initial state={}", server.state_label());

    let listening = server.listen();
    println!("after bind state={}", listening.state_label());

    let established = listening.accept("192.168.0.10:55000")?;
    println!("after accept state={}", established.state_label());
    established.send(b"HTTP/1.1 200 OK");

    // 次の行のコメントを外すとコンパイルエラー:
    // `send` は Established にしか実装されていない。
    // let server = TcpConnection::<Closed>::new("127.0.0.1:8080")?;
    // server.send(b"oops");

    // `ConnectionState` は sealed なので、外部 crate は勝手な状態を追加できない。

    Ok(())
}
