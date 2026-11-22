use alloc::{sync::Arc, vec, vec::Vec};
use core::{
    fmt::{self, Display, Formatter},
    net::SocketAddr,
    num::{NonZeroU64, NonZeroUsize},
    primitive::{bool, u64},
    result::Result::{self, Err, Ok},
    time::Duration,
};
use std::{
    eprintln,
    io::{self, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    Error, Output,
    file_store::FsStore,
    server::{
        Open,
        app::App,
        event::{BODY_SIZE_MAX, Event},
        handler::reject,
        http::{self, Response},
    },
};

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
    Error(Error),
    Response(Response),
}

#[derive(Clone, Copy)]
pub struct Config {
    pub address:                   SocketAddr,
    pub connection_thread_count:   NonZeroUsize,
    pub http_header_count_max:     NonZeroUsize,
    pub http_header_size_max:      NonZeroUsize,
    pub maintain_interval_seconds: NonZeroU64,
    pub read_size:                 NonZeroUsize,
    pub read_timeout_seconds:      NonZeroU64,
    pub write_timeout_seconds:     NonZeroU64,
}

pub fn serve(config: Config, app: Arc<Mutex<App<FsStore>>>) -> Result<(), HostError> {
    let listener = Arc::new(TcpListener::bind(config.address).map_err(HostError::Bind)?);
    let mut handles = Vec::with_capacity(config.connection_thread_count.get());
    for _ in 0..config.connection_thread_count.get() {
        let listener = Arc::clone(&listener);
        let app = Arc::clone(&app);
        let handle = thread::Builder::new()
            .spawn(move || {
                loop {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let _ = serve_connection(stream, &app, &config);
                        }
                        Err(error) if error.kind() == ErrorKind::ConnectionAborted => {}
                        Err(error) => return Err(HostError::Accept(error)),
                    }
                }
            })
            .map_err(HostError::Spawn)?;
        handles.push(handle);
    }
    loop {
        thread::sleep(Duration::from_secs(config.maintain_interval_seconds.get()));
        let Ok(mut app) = app.lock() else {
            return Err(HostError::Panicked);
        };
        app.run(now(), Event::Maintain);
        for command in app.take_commands() {
            if let Command::Error(error) = command {
                eprintln!("maintain: {error}");
            }
        }
        drop(app);
        if let Some(index) = handles.iter().position(JoinHandle::is_finished) {
            return match handles.swap_remove(index).join() {
                Ok(result) => result,
                Err(_) => Err(HostError::Panicked),
            };
        }
    }
}

fn serve_connection<S: Open>(
    mut stream: TcpStream,
    app: &Mutex<App<S>>,
    config: &Config,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(config.read_timeout_seconds.get())))?;
    stream.set_write_timeout(Some(Duration::from_secs(config.write_timeout_seconds.get())))?;
    stream.set_nodelay(true)?;
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        let (commands, keep_alive, consumed) = match http::frame(
            &buffer,
            BODY_SIZE_MAX,
            config.http_header_count_max,
            config.http_header_size_max,
        ) {
            Ok(Some(frame)) => {
                (process_event(app, &buffer[..frame.length]), frame.keep_alive, frame.length)
            }
            Ok(None) => {
                let length = buffer.len();
                buffer.resize(length + config.read_size.get(), 0);
                let count = stream.read(&mut buffer[length..])?;
                buffer.truncate(length + count);
                if count == 0 {
                    return Ok(());
                }
                continue;
            }
            Err(error) => {
                (vec![Command::Response(reject(Error::Http(error)))], false, buffer.len())
            }
        };
        drain(&mut stream, commands, keep_alive)?;
        if !keep_alive {
            return Ok(());
        }
        buffer.drain(..consumed);
    }
}

fn process_event<S: Open>(app: &Mutex<App<S>>, frame: &[u8]) -> Vec<Command> {
    match app.lock() {
        Ok(mut app) => {
            app.process(now(), frame);
            app.take_commands()
        }
        Err(_) => vec![Command::Response(Response::new(500))],
    }
}

fn drain(stream: &mut TcpStream, commands: Vec<Command>, keep_alive: bool) -> io::Result<()> {
    for command in commands {
        execute(stream, command, keep_alive)?;
    }
    stream.flush()
}

fn execute(stream: &mut TcpStream, command: Command, keep_alive: bool) -> io::Result<()> {
    match command {
        Command::Error(error) => {
            eprintln!("request: {error}");
            Ok(())
        }
        Command::Response(response) => {
            let connection = if keep_alive { "keep-alive" } else { "close" };
            stream.write_all(&response.header("Connection", connection).to_bytes())
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, string::String};
    use std::net::Shutdown;

    use super::*;
    use crate::{
        file_store::MemoryStore,
        server::handler::{self, Handler},
    };

    fn exchange(request: &[u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let request = request.to_vec();
        let client = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            stream.write_all(&request).unwrap();
            stream.shutdown(Shutdown::Write).unwrap();
            let mut received = Vec::new();
            stream.read_to_end(&mut received).unwrap();
            received
        });
        let handler = Handler::<MemoryStore>::open(
            (),
            0,
            b"pepper".to_vec(),
            BTreeSet::new(),
            handler::Config::new(),
        )
        .unwrap();
        let app = Mutex::new(App::new(handler));
        let (stream, _) = listener.accept().unwrap();
        let config = Config {
            address,
            connection_thread_count: NonZeroUsize::new(1).unwrap(),
            http_header_count_max: NonZeroUsize::new(32).unwrap(),
            http_header_size_max: NonZeroUsize::new(8 * 1024).unwrap(),
            maintain_interval_seconds: NonZeroU64::new(60).unwrap(),
            read_size: NonZeroUsize::new(16 * 1024).unwrap(),
            read_timeout_seconds: NonZeroU64::new(10).unwrap(),
            write_timeout_seconds: NonZeroU64::new(10).unwrap(),
        };
        serve_connection(stream, &app, &config).unwrap();
        String::from_utf8_lossy(&client.join().unwrap()).into_owned()
    }

    fn statuses(received: &str) -> Vec<&str> {
        received.match_indices("HTTP/1.1 ").map(|(at, _)| &received[at + 9..at + 12]).collect()
    }

    #[test]
    fn pipelined_requests_are_answered_in_order_until_the_connection_closes() {
        let received = exchange(
            b"GET /v1/health HTTP/1.1\r\n\r\nGET /v1/nothing HTTP/1.1\r\n\r\nGET /v1/health HTTP/1.0\r\n\r\nGET /v1/health HTTP/1.1\r\n\r\n",
        );
        assert_eq!(statuses(&received), ["200", "400", "200"]);
        assert_eq!(received.matches("Connection: keep-alive").count(), 2);
        assert_eq!(received.matches("Connection: close").count(), 1);
    }

    #[test]
    fn a_request_that_cannot_be_delimited_is_rejected_and_closed() {
        let received = exchange(b"GET / HTTP/2\r\n\r\nGET /v1/health HTTP/1.1\r\n\r\n");
        assert_eq!(statuses(&received), ["505"]);
        assert!(received.contains("Connection: close"));
    }
}
