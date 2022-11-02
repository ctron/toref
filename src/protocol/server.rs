use bytes::{Buf, BytesMut};
use std::io::{Cursor, ErrorKind};
use std::str::from_utf8;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::{io, net::TcpListener};

const TYPE_STRING: u8 = 80;

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub async fn new<A: ToSocketAddrs>(address: A) -> io::Result<Self> {
        let listener = TcpListener::bind(address).await?;
        Ok(Self { listener })
    }

    pub async fn run(self) -> io::Result<()> {
        loop {
            let (stream, addr) = self.listener.accept().await?;
            log::info!("New connection: {addr}");

            let connection = Connection::new(stream);

            tokio::spawn(async {
                match connection.run().await {
                    Ok(_) => log::info!("Connection closed"),
                    Err(err) => log::warn!("Connection closed: {err}"),
                }
            });
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub dest: String,
    pub request: Request,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Request {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Action")]
    action: Action,
    #[serde(rename = "$value")]
    data: Data,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Action {
    Create,
    Delete,
    Start,
    Stop,
    Kill,
    Query,
    Read,
    Write,
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Data {
    #[serde(rename = "FB")]
    #[serde(rename_all = "PascalCase")]
    FunctionBlock {
        name: String,
        r#type: String,
    },
    Watches,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Response {
    #[serde(rename = "ID")]
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<Error>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "$value")]
    data: Option<Data>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, thiserror::Error)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Error {
    #[error("not ready")]
    NotReady,
    #[error("supported command")]
    #[serde(rename = "UNSUPPORTED_CMD")]
    UnsupportedCommand,
    #[error("supported type")]
    UnsupportedType,
    #[error("no such object")]
    NoSuchObject,
    #[error("invalid object")]
    InvalidObject,
    #[error("invalid operation")]
    InvalidOperation,
    #[error("invalid state")]
    InvalidState,
    #[error("overflow")]
    Overflow,
    #[error("duplicate object")]
    DuplicateObject,
    #[error("invalid destination")]
    #[serde(rename = "INVALID_DST")]
    InvalidDestination,
    #[error("null pointer")]
    NullPointer,
    #[error("interrupted")]
    Interrupted,
    #[error("unknown")]
    Unknown,
}

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(64 * 1024 + 1024),
        }
    }

    async fn run(mut self) -> io::Result<()> {
        loop {
            let req = self.read_request().await?;
            match req {
                Some(req) => {
                    log::info!("Request: {req:?}");
                    match self
                        .process_request(req.dest, req.request.action, req.request.data)
                        .await
                    {
                        Ok(data) => {
                            self.write_response(Response {
                                id: req.request.id,
                                reason: None,
                                data,
                            })
                            .await?;
                        }
                        Err(err) => {
                            self.write_response(Response {
                                id: req.request.id,
                                reason: Some(err),
                                data: None,
                            })
                            .await?;
                        }
                    }
                }
                None => {
                    // orderly shutdown
                    return Ok(());
                }
            }
        }
    }

    async fn process_request(
        &mut self,
        destination: String,
        action: Action,
        data: Data,
    ) -> Result<Option<Data>, Error> {
        if !destination.is_empty() {
            return Err(Error::InvalidDestination);
        }

        Ok(match (action, data) {
            (Action::Read, Data::Watches) => None,
            (Action::Query, Data::FunctionBlock { .. }) => None,
            _ => return Err(Error::InvalidOperation),
        })
    }

    async fn write_response(&mut self, response: Response) -> io::Result<()> {
        self.stream.write_u8(TYPE_STRING).await?;

        let buf = quick_xml::se::to_string(&response).map_err(|err| {
            io::Error::new(
                ErrorKind::Other,
                format!("Failed to encode response: {err}"),
            )
        })?;
        if buf.len() > u16::MAX as usize {
            return Err(io::Error::new(
                ErrorKind::OutOfMemory,
                "Response too larger",
            ));
        }

        self.stream.write_u16(buf.len() as u16).await?;
        self.stream.write_all(buf.as_bytes()).await?;
        self.stream.flush().await?;

        Ok(())
    }

    async fn read_request(&mut self) -> io::Result<Option<Frame>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                return if self.buffer.is_empty() {
                    // graceful disconnect
                    Ok(None)
                } else {
                    // in-frame disconnect
                    Err(io::Error::new(
                        ErrorKind::ConnectionReset,
                        "Connection reset by peer",
                    ))
                };
            }
        }
    }

    fn parse_frame(&mut self) -> io::Result<Option<Frame>> {
        let mut buf = Cursor::new(&self.buffer[..]);

        // FIXME: we could do better by checking the data first, before "reading" strings

        let dest = match Self::read_string(&mut buf)? {
            None => {
                return Ok(None);
            }
            Some(dest) => dest,
        };

        match Self::read_string(&mut buf)? {
            None => Ok(None),
            Some(data) => {
                let len = buf.position() as usize;
                log::debug!("Request was {len} bytes");
                self.buffer.advance(buf.position() as usize);
                let request = quick_xml::de::from_str(&data).map_err(|err| {
                    io::Error::new(
                        ErrorKind::InvalidData,
                        format!("Failed to decode XML: {err}"),
                    )
                })?;
                Ok(Some(Frame { dest, request }))
            }
        }
    }

    fn read_string(buf: &mut Cursor<&[u8]>) -> io::Result<Option<String>> {
        if buf.remaining() < 1 + 2 {
            return Ok(None);
        }

        let r#type = buf.get_u8();
        if r#type != TYPE_STRING {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Illegal data type: {type:#02x}"),
            ));
        }

        let len = buf.get_u16() as usize;

        if buf.remaining() < len {
            return Ok(None);
        }

        let start = buf.position() as usize;
        let end = start + len;

        buf.set_position(end as u64);

        from_utf8(&buf.get_ref()[start..end])
            .map(|s| Some(s.to_string()))
            .map_err(|err| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Failed to decode string: {err}"),
                )
            })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn decode() {
        let request: Request = quick_xml::de::from_str(
            r#"
        <Request ID="1" Action="QUERY">
            <FB Name="*" Type="*"/>
        </Request>
        "#,
        )
        .unwrap();
    }
}
