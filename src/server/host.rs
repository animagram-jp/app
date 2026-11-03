use alloc::{sync::Arc, vec::Vec};
use core::{
    fmt::{self, Display, Formatter},
    net::SocketAddr,
    primitive::{u64, usize},
    result::Result::{self, Err, Ok},
    time::Duration,
};
use std::{
    io::{self, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    Error,
    file_store::FsStore,
    server::{
        handler::{self, Handler},
        http,
    },
};

pub const READ_TIMEOUT: Duration = Duration::from_secs(10);
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
pub const BUFFER_SIZE_MAX: usize = http::HEADER_SIZE_MAX + http::BODY_MAX;
pub const CHUNK_SIZE: usize = 16 * 1024;
pub const MAINTAIN_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum HostError {
    Bind(io::Error),
    Accept(io::Error),
    Spawn(io::Error),
    Panicked,
}

impl Display for HostError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs()).unwrap_or(0)
}

pub struct Config {
    pub address:      SocketAddr,
    pub thread_count: usize,
}

pub fn serve(config: Config, handler: Arc<Mutex<Handler<FsStore>>>) -> Result<(), HostError> {
    let listener = Arc::new(TcpListener::bind(config.address).map_err(HostError::Bind)?);
    let mut handles = Vec::with_capacity(config.thread_count);
    let keeper = Arc::clone(&handler);
    thread::Builder::new()
        .spawn(move || {
            loop {
                thread::sleep(MAINTAIN_INTERVAL);
                if let Ok(mut handler) = keeper.lock() {
                    let _ = handler.maintain(now());
                }
            }
        })
        .map_err(HostError::Spawn)?;
    for _ in 0..config.thread_count.max(1) {
        let listener = Arc::clone(&listener);
        let handler = Arc::clone(&handler);
        let handle = thread::Builder::new()
            .spawn(move || {
                loop {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let _ = serve_connection(stream, &handler);
                        }
                        Err(error) if error.kind() == ErrorKind::ConnectionAborted => {}
                        Err(error) => return Err(HostError::Accept(error)),
                    }
                }
            })
            .map_err(HostError::Spawn)?;
        handles.push(handle);
    }
    for handle in handles {
        match handle.join() {
            Ok(result) => result?,
            Err(_) => return Err(HostError::Panicked),
        }
    }
    Ok(())
}

fn serve_connection(mut stream: TcpStream, handler: &Mutex<Handler<FsStore>>) -> io::Result<()> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
    stream.set_nodelay(true)?;
    let mut buffer: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
    let mut chunk = [0u8; CHUNK_SIZE];
    loop {
        let (response, keep_alive, consumed) = match http::parse(&buffer) {
            Ok(Some((request, consumed))) => {
                (respond(handler, &request), request.keep_alive, consumed)
            }
            Ok(None) => {
                if buffer.len() >= BUFFER_SIZE_MAX {
                    (
                        handler::reject(Error::Http(http::HttpError::BodyTooLarge)),
                        false,
                        buffer.len(),
                    )
                } else {
                    let count = stream.read(&mut chunk)?;
                    if count == 0 {
                        return Ok(());
                    }
                    buffer.extend_from_slice(&chunk[..count]);
                    continue;
                }
            }
            Err(error) => (handler::reject(Error::Http(error)), false, buffer.len()),
        };
        stream.write_all(&response.to_bytes(keep_alive))?;
        stream.flush()?;
        if !keep_alive {
            return Ok(());
        }
        buffer.drain(..consumed);
    }
}

fn respond(handler: &Mutex<Handler<FsStore>>, request: &http::Request) -> http::Response {
    match handler.lock() {
        Ok(mut handler) => handler.process_request(now(), request),
        Err(_) => http::Response::new(500),
    }
}
