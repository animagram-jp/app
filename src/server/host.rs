use alloc::{sync::Arc, vec, vec::Vec};
use core::{
    fmt::{self, Display, Formatter},
    net::SocketAddr,
    primitive::{bool, u32, u64, usize},
    result::Result::{self, Err, Ok},
    sync::atomic::{AtomicU32, Ordering},
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
    Output,
    file_store::FsStore,
    server::{
        app::App,
        event::{Chunk, Event},
        http::Response,
    },
};

pub const READ_TIMEOUT: Duration = Duration::from_secs(10);
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
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

pub enum Command {
    Response { connection: u32, response: Response },
    Close { connection: u32 },
}

pub struct Config {
    pub address:      SocketAddr,
    pub thread_count: usize,
}

static CONNECTION_COUNT: AtomicU32 = AtomicU32::new(0);

pub fn serve(config: Config, app: Arc<Mutex<App<FsStore>>>) -> Result<(), HostError> {
    let listener = Arc::new(TcpListener::bind(config.address).map_err(HostError::Bind)?);
    let mut handles = Vec::with_capacity(config.thread_count);
    let keeper = Arc::clone(&app);
    thread::Builder::new()
        .spawn(move || {
            loop {
                thread::sleep(MAINTAIN_INTERVAL);
                process_event(&keeper, Event::Maintain);
            }
        })
        .map_err(HostError::Spawn)?;
    for _ in 0..config.thread_count.max(1) {
        let listener = Arc::clone(&listener);
        let app = Arc::clone(&app);
        let handle = thread::Builder::new()
            .spawn(move || {
                loop {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let _ = serve_connection(stream, &app);
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

fn serve_connection(mut stream: TcpStream, app: &Mutex<App<FsStore>>) -> io::Result<()> {
    let connection = CONNECTION_COUNT.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    let served = (|| {
        stream.set_read_timeout(Some(READ_TIMEOUT))?;
        stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
        stream.set_nodelay(true)?;
        let mut chunk = [0u8; CHUNK_SIZE];
        loop {
            let count = stream.read(&mut chunk)?;
            if count == 0 {
                return Ok(());
            }
            let bytes = chunk[..count].to_vec();
            if !drain(&mut stream, process_event(app, Event::Chunk(Chunk { connection, bytes })))? {
                return Ok(());
            }
        }
    })();
    process_event(app, Event::Closed(connection));
    served
}

fn process_event(app: &Mutex<App<FsStore>>, event: Event) -> Vec<Command> {
    match app.lock() {
        Ok(mut app) => {
            app.process(now(), event);
            app.take_commands()
        }
        Err(_) => match event {
            Event::Chunk(Chunk { connection, .. }) => vec![
                Command::Response { connection, response: Response::new(500) },
                Command::Close { connection },
            ],
            _ => Vec::new(),
        },
    }
}

fn drain(stream: &mut TcpStream, commands: Vec<Command>) -> io::Result<bool> {
    for command in commands {
        if !execute(stream, command)? {
            stream.flush()?;
            return Ok(false);
        }
    }
    stream.flush()?;
    Ok(true)
}

fn execute(stream: &mut TcpStream, command: Command) -> io::Result<bool> {
    match command {
        Command::Response { response, .. } => {
            stream.write_all(&response.to_bytes())?;
            Ok(true)
        }
        Command::Close { .. } => Ok(false),
    }
}
