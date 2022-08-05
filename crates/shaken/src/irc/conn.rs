use std::sync::Arc;

use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufStream},
    net::TcpStream,
};

use anyhow::Context as _;

use super::{Identity, Privmsg, Tags};

pub struct Conn {
    pub(in crate::irc) stream: BufStream<TcpStream>,
    pub(in crate::irc) buf: String,
}

impl Conn {
    pub async fn join_channel(&mut self, channel: &str) -> anyhow::Result<()> {
        self.stream
            .write_all(format!("JOIN {channel}\r\n").as_bytes())
            .await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn read_message(&mut self) -> anyhow::Result<Privmsg> {
        loop {
            self.buf.clear();

            let n = self.stream.read_line(&mut self.buf).await?;
            let line = &self.buf[..n];

            let (tags, prefix, cmd, args, data) = Self::parse(line);
            let prefix = prefix.map(Arc::<str>::from);
            let data = data.map(Arc::<str>::from);

            match cmd {
                "PING" => {
                    let resp = format!("PONG :{}\r\n", data.unwrap());
                    self.stream.write_all(resp.as_bytes()).await?;
                    self.stream.flush().await?;
                }
                "ERROR" => anyhow::bail!("error: {:?}", data),
                "PRIVMSG" => {
                    return Ok(Privmsg {
                        tags,
                        user: prefix.expect("prefix attached"),
                        target: args[0].into(),
                        data: data.expect("malformed message"),
                    });
                }
                _ => {}
            }
        }
    }

    pub async fn privmsg(&mut self, target: &str, data: &str) -> anyhow::Result<()> {
        let data = format!("PRIVMSG {target} :{data}\r\n");
        self.stream.write_all(data.as_bytes()).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub(in crate::irc) async fn wait_for_ready(
        default_name: &str,
        buf: &mut String,
        stream: &mut BufStream<TcpStream>,
    ) -> anyhow::Result<Identity> {
        loop {
            let n = stream.read_line(buf).await?;
            let mut raw = &buf[..n - 2];

            let tags = raw
                .starts_with('@')
                .then(|| Tags::parse(&mut raw))
                .flatten()
                .unwrap_or_default();

            match raw.split_once(' ') {
                Some(("PING", tail)) => {
                    let token = tail
                        .rsplit_terminator(':')
                        .next()
                        .with_context(|| "PING must have a token")?;
                    let out = format!("PONG :{token}\r\n");
                    stream.write_all(out.as_bytes()).await?;
                }
                Some((.., "GLOBALUSERSTATE")) => {
                    let name = tags.get("display-name").unwrap_or(default_name).into();
                    let user_id = tags.get_parsed("user-id")?;
                    let identity = Identity { name, user_id };
                    return Ok(identity);
                }
                Some(("ERROR", tail)) => anyhow::bail!("{tail}"),
                _ => {}
            }

            buf.clear();
        }
    }

    fn prefix<'a>(input: &mut &'a str) -> Option<&'a str> {
        if input.starts_with(':') {
            let (head, tail) = input.split_once(' ').expect("malformed message");
            *input = tail;
            return head[1..].split_terminator('!').next();
        }
        None
    }

    fn command<'a>(input: &mut &'a str) -> &'a str {
        // TODO we got a panic ehre
        let (head, tail) = input.split_once(' ').expect("malformed message");
        *input = tail;
        head
    }

    fn args<'a>(input: &mut &'a str) -> Vec<&'a str> {
        if let Some((head, tail)) = input.split_once(':') {
            *input = tail;
            head.split_ascii_whitespace().collect()
        } else {
            vec![]
        }
    }

    fn data<'a>(input: &mut &'a str) -> Option<&'a str> {
        Some(input.trim_end()).filter(|s| !s.is_empty())
    }

    fn parse(mut line: &str) -> (Tags, Option<&str>, &str, Vec<&str>, Option<&str>) {
        let line = &mut line;
        let tags = if line.starts_with('@') {
            Tags::parse(line)
        } else {
            None
        }
        .unwrap_or_default();

        let prefix = if line.starts_with(':') {
            Self::prefix(line).map(Into::into)
        } else {
            None
        };

        let command = Self::command(line);
        let args = Self::args(line);
        let data = Self::data(line).map(Into::into);

        (tags, prefix, command, args, data)
    }

    /*
    pub async fn write_raw(&self, data: impl AsRef<[u8]>) -> std::io::Result<()> {
        { &self.write }.write_all(data.as_ref()).await?;
        { &self.write }.write_all(b"\r\n").await?;
        { &self.write }.flush().await?;
        Ok(())
    }

    pub fn try_read(&mut self) -> std::io::Result<ReadState<&str>> {
        let would_block = |e| {
            use std::io::ErrorKind::*;
            matches!(e, WouldBlock | Interrupted)
        };

        match self.read.read_line(&mut self.buf) {
            Ok(0) => Ok(ReadState::Eof),
            Ok(n) => Ok(ReadState::Complete(&self.buf[..n])),
            Err(e) if would_block(e.kind()) => Ok(ReadState::Incomplete),
            Err(e) => Err(e),
        }
    }

    pub fn wait_for_ready(
        default_name: &str,
        buf: &mut String,
        read: &mut impl BufRead,
        write: &mut impl Write,
    ) -> anyhow::Result<Identity> {
        loop {
            buf.clear();
            let n = match read.read_line(buf)? {
                0 => anyhow::bail!("unexpected EOF"),
                n => n,
            };

            let mut raw = buf[..n].trim_end();
            let tags = raw
                .starts_with('@')
                .then(|| Tags::parse(&mut raw))
                .flatten()
                .unwrap_or_default();

            match raw.split_once(' ') {
                Some(("PING", tail)) => {
                    let token = tail
                        .rsplit_terminator(':')
                        .next()
                        .with_context(|| "PING must have a token")?;
                    write.write_fmt(format_args!("PONG :{token}\r\n"))?;
                }
                Some((.., "GLOBALUSERSTATE")) => {
                    let name = tags.get("display-name").unwrap_or(default_name).into();
                    let user_id = tags.get_parsed("user-id")?;
                    let identity = Identity { name, user_id };
                    break Ok(identity);
                }
                Some(("ERROR", tail)) => anyhow::bail!("{tail}"),
                _ => {}
            }
        }
    }
    */
}
