pub const TWITCH_NO_TLS: &str = "irc.chat.twitch.tv:6667";

mod tags;
pub use tags::Tags;

mod conn;
pub use conn::Conn;

mod privmsg;
pub use privmsg::Privmsg;

#[derive(Copy, Clone)]
pub struct Registration<'a> {
    pub name: &'a str,
    pub pass: &'a str,
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub name: Box<str>,
    pub user_id: u64,
}

pub async fn connect(addr: &str, reg: Registration<'_>) -> anyhow::Result<(Identity, Conn)> {
    use tokio::{
        io::{AsyncWriteExt as _, BufStream},
        net::TcpStream,
    };

    let mut stream = TcpStream::connect(addr).await?;

    for cap in [
        "CAP REQ :twitch.tv/membership\r\n",
        "CAP REQ :twitch.tv/tags\r\n",
        "CAP REQ :twitch.tv/commands\r\n",
    ] {
        stream.write_all(cap.as_bytes()).await?;
    }
    stream.flush().await?;

    let Registration { name, pass, .. } = reg;
    for reg in [format!("PASS {pass}\r\n"), format!("NICK {name}\r\n")] {
        stream.write_all(reg.as_bytes()).await?;
    }
    stream.flush().await?;

    let mut stream = BufStream::new(stream);
    let mut buf = String::with_capacity(1024);

    let identity = Conn::wait_for_ready(name, &mut buf, &mut stream).await?;
    buf.clear();

    Ok((identity, Conn { stream, buf }))
}
