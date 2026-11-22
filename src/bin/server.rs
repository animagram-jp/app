extern crate alloc;

use alloc::sync::Arc;
use core::{
    net::SocketAddr,
    num::{NonZeroU16, NonZeroU64, NonZeroUsize},
};
use std::{env, fs, path::PathBuf, process::ExitCode, sync::Mutex};

use app::{
    file_store::FsStore,
    server::{
        account::parse_allowlist,
        app::App,
        handler::{self, Handler},
        host::{self, now, serve},
        session::{CookieName, CookiePath},
    },
};

const PEPPER_LENGTH_MIN: usize = 32;

fn main() -> ExitCode {
    let address: SocketAddr = match env::var("SERVER_ADDRESS")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(address)) => address,
        Some(Err(error)) => {
            eprintln!("SERVER_ADDRESS: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_ADDRESS: bind address is required");
            return ExitCode::from(2);
        }
    };
    let allowlist = match env::var("SERVER_ALLOWLIST_PATH")
        .ok()
        .filter(|value| !value.is_empty())
        .map(fs::read_to_string)
    {
        Some(Ok(text)) => parse_allowlist(&text),
        Some(Err(error)) => {
            eprintln!("SERVER_ALLOWLIST_PATH: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_ALLOWLIST_PATH: path to the allowlist file is required");
            return ExitCode::from(2);
        }
    };
    let compact_interval_seconds: NonZeroU64 = match env::var("SERVER_COMPACT_INTERVAL_SECONDS")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(compact_interval_seconds)) => compact_interval_seconds,
        Some(Err(error)) => {
            eprintln!("SERVER_COMPACT_INTERVAL_SECONDS: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_COMPACT_INTERVAL_SECONDS: compact interval is required");
            return ExitCode::from(2);
        }
    };
    let connection_thread_count: NonZeroUsize = match env::var("SERVER_CONNECTION_THREAD_COUNT")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(connection_thread_count)) => connection_thread_count,
        Some(Err(error)) => {
            eprintln!("SERVER_CONNECTION_THREAD_COUNT: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_CONNECTION_THREAD_COUNT: connection thread count is required");
            return ExitCode::from(2);
        }
    };
    let data_directory: PathBuf =
        match env::var("SERVER_DATA_PATH").ok().filter(|value| !value.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => {
                eprintln!("SERVER_DATA_PATH: path to the data directory is required");
                return ExitCode::from(2);
            }
        };
    let http_header_count_max: NonZeroUsize = match env::var("SERVER_HTTP_HEADER_COUNT_MAX")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(http_header_count_max)) => http_header_count_max,
        Some(Err(error)) => {
            eprintln!("SERVER_HTTP_HEADER_COUNT_MAX: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_HTTP_HEADER_COUNT_MAX: header count limit is required");
            return ExitCode::from(2);
        }
    };
    let http_header_size_max: NonZeroUsize = match env::var("SERVER_HTTP_HEADER_SIZE_MAX")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(http_header_size_max)) => http_header_size_max,
        Some(Err(error)) => {
            eprintln!("SERVER_HTTP_HEADER_SIZE_MAX: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_HTTP_HEADER_SIZE_MAX: header size limit is required");
            return ExitCode::from(2);
        }
    };
    let maintain_interval_seconds: NonZeroU64 = match env::var("SERVER_MAINTAIN_INTERVAL_SECONDS")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(maintain_interval_seconds)) => maintain_interval_seconds,
        Some(Err(error)) => {
            eprintln!("SERVER_MAINTAIN_INTERVAL_SECONDS: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_MAINTAIN_INTERVAL_SECONDS: maintain interval is required");
            return ExitCode::from(2);
        }
    };
    let meet_lifetime_seconds: NonZeroU64 = match env::var("SERVER_MEET_LIFETIME_SECONDS")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(meet_lifetime_seconds)) => meet_lifetime_seconds,
        Some(Err(error)) => {
            eprintln!("SERVER_MEET_LIFETIME_SECONDS: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_MEET_LIFETIME_SECONDS: signal lifetime is required");
            return ExitCode::from(2);
        }
    };
    let meet_presence_seconds: NonZeroU64 = match env::var("SERVER_MEET_PRESENCE_SECONDS")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(meet_presence_seconds)) => meet_presence_seconds,
        Some(Err(error)) => {
            eprintln!("SERVER_MEET_PRESENCE_SECONDS: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_MEET_PRESENCE_SECONDS: presence duration is required");
            return ExitCode::from(2);
        }
    };
    let meet_queue_max: NonZeroU16 = match env::var("SERVER_MEET_QUEUE_MAX")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(meet_queue_max)) => meet_queue_max,
        Some(Err(error)) => {
            eprintln!("SERVER_MEET_QUEUE_MAX: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_MEET_QUEUE_MAX: queue limit is required");
            return ExitCode::from(2);
        }
    };
    let pepper =
        match env::var("SERVER_PEPPER_PATH").ok().filter(|value| !value.is_empty()).map(fs::read) {
            Some(Ok(pepper)) if pepper.len() < PEPPER_LENGTH_MIN => {
                eprintln!("SERVER_PEPPER_PATH: at least {PEPPER_LENGTH_MIN} bytes required");
                return ExitCode::from(2);
            }
            Some(Ok(pepper)) if pepper.iter().all(|byte| *byte == 0) => {
                eprintln!("SERVER_PEPPER_PATH: all bytes are zero");
                return ExitCode::from(2);
            }
            Some(Ok(pepper)) => pepper,
            Some(Err(error)) => {
                eprintln!("SERVER_PEPPER_PATH: {error}");
                return ExitCode::from(2);
            }
            None => {
                eprintln!("SERVER_PEPPER_PATH: path to the pepper file is required");
                return ExitCode::from(2);
            }
        };
    let read_size: NonZeroUsize = match env::var("SERVER_READ_SIZE")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(read_size)) => read_size,
        Some(Err(error)) => {
            eprintln!("SERVER_READ_SIZE: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_READ_SIZE: read size is required");
            return ExitCode::from(2);
        }
    };
    let read_timeout_seconds: NonZeroU64 = match env::var("SERVER_READ_TIMEOUT_SECONDS")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(read_timeout_seconds)) => read_timeout_seconds,
        Some(Err(error)) => {
            eprintln!("SERVER_READ_TIMEOUT_SECONDS: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_READ_TIMEOUT_SECONDS: read timeout is required");
            return ExitCode::from(2);
        }
    };
    let session_cookie_name = match env::var("SERVER_SESSION_COOKIE_NAME")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| CookieName::parse(&value))
    {
        Some(Some(session_cookie_name)) => session_cookie_name,
        Some(None) => {
            eprintln!("SERVER_SESSION_COOKIE_NAME: cookie name must be a token");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_SESSION_COOKIE_NAME: cookie name is required");
            return ExitCode::from(2);
        }
    };
    let session_cookie_path = match env::var("SERVER_SESSION_COOKIE_PATH")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| CookiePath::parse(&value))
    {
        Some(Some(session_cookie_path)) => session_cookie_path,
        Some(None) => {
            eprintln!(
                "SERVER_SESSION_COOKIE_PATH: cookie path must start with / and contain no control character or ;"
            );
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_SESSION_COOKIE_PATH: cookie path is required");
            return ExitCode::from(2);
        }
    };
    let session_inactivity_seconds: NonZeroU64 = match env::var("SERVER_SESSION_INACTIVITY_SECONDS")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(session_inactivity_seconds)) => session_inactivity_seconds,
        Some(Err(error)) => {
            eprintln!("SERVER_SESSION_INACTIVITY_SECONDS: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_SESSION_INACTIVITY_SECONDS: inactivity limit is required");
            return ExitCode::from(2);
        }
    };
    let session_lifetime_seconds: NonZeroU64 = match env::var("SERVER_SESSION_LIFETIME_SECONDS")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(session_lifetime_seconds)) => session_lifetime_seconds,
        Some(Err(error)) => {
            eprintln!("SERVER_SESSION_LIFETIME_SECONDS: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_SESSION_LIFETIME_SECONDS: session lifetime is required");
            return ExitCode::from(2);
        }
    };
    let write_timeout_seconds: NonZeroU64 = match env::var("SERVER_WRITE_TIMEOUT_SECONDS")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse())
    {
        Some(Ok(write_timeout_seconds)) => write_timeout_seconds,
        Some(Err(error)) => {
            eprintln!("SERVER_WRITE_TIMEOUT_SECONDS: {error}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("SERVER_WRITE_TIMEOUT_SECONDS: write timeout is required");
            return ExitCode::from(2);
        }
    };
    if let Err(error) = fs::create_dir_all(&data_directory) {
        eprintln!("SERVER_DATA_PATH: {error}");
        return ExitCode::from(2);
    }
    let handler = match Handler::<FsStore>::open(
        data_directory,
        now(),
        pepper,
        allowlist,
        handler::Config {
            compact_interval_seconds,
            meet_lifetime_seconds,
            meet_presence_seconds,
            meet_queue_max,
            session_cookie_name,
            session_cookie_path,
            session_inactivity_seconds,
            session_lifetime_seconds,
        },
    ) {
        Ok(handler) => handler,
        Err(error) => {
            eprintln!("open: {error}");
            return ExitCode::FAILURE;
        }
    };
    match serve(
        host::Config {
            address,
            connection_thread_count,
            http_header_count_max,
            http_header_size_max,
            maintain_interval_seconds,
            read_size,
            read_timeout_seconds,
            write_timeout_seconds,
        },
        Arc::new(Mutex::new(App::new(handler))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("server: {error}");
            ExitCode::FAILURE
        }
    }
}
