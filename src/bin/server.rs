use std::{
    env, fs,
    net::SocketAddr,
    path::PathBuf,
    process::ExitCode,
    sync::{Arc, Mutex},
};

use app::{
    file_store::FsStore,
    server::{
        account::parse_allowlist,
        handler::Handler,
        host::{Config, now, serve},
    },
};

const ADDRESS: &str = "127.0.0.1:2001";
const THREAD_COUNT: usize = 4;
const PEPPER_LENGTH_MIN: usize = 32;
const ALLOWLIST: &str = "allowlist";
const PEPPER_CREDENTIAL: &str = "pepper";

fn main() -> ExitCode {
    let address: SocketAddr =
        match env::var("SERVER_ADDRESS").unwrap_or_else(|_| ADDRESS.to_string()).parse() {
            Ok(address) => address,
            Err(error) => {
                eprintln!("SERVER_ADDRESS: {error}");
                return ExitCode::from(2);
            }
        };
    let thread_count = env::var("SERVER_WORKERS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(THREAD_COUNT);
    let pepper_path = env::var("SERVER_PEPPER").map(PathBuf::from).or_else(|_| {
        env::var("CREDENTIALS_DIRECTORY")
            .map(|directory| PathBuf::from(directory).join(PEPPER_CREDENTIAL))
    });
    let pepper = match pepper_path.map(fs::read) {
        Ok(Ok(pepper)) if pepper.len() >= PEPPER_LENGTH_MIN => pepper,
        Ok(Ok(_)) => {
            eprintln!("SERVER_PEPPER: at least {PEPPER_LENGTH_MIN} bytes required");
            return ExitCode::from(2);
        }
        Ok(Err(error)) => {
            eprintln!("SERVER_PEPPER: {error}");
            return ExitCode::from(2);
        }
        Err(_) => {
            eprintln!("SERVER_PEPPER: path to a secret file or a systemd credential is required");
            return ExitCode::from(2);
        }
    };
    let data_directory: PathBuf = match env::var("SERVER_DATA") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            eprintln!("SERVER_DATA: path to the data directory is required");
            return ExitCode::from(2);
        }
    };
    if let Err(error) = fs::create_dir_all(&data_directory) {
        eprintln!("SERVER_DATA: {error}");
        return ExitCode::from(2);
    }
    let allowlist =
        parse_allowlist(&fs::read_to_string(data_directory.join(ALLOWLIST)).unwrap_or_default());
    let origins: Vec<String> = env::var("SERVER_ORIGIN")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let handler = match Handler::<FsStore>::open(data_directory, now(), pepper, allowlist, origins)
    {
        Ok(handler) => handler,
        Err(error) => {
            eprintln!("open: {error}");
            return ExitCode::FAILURE;
        }
    };
    match serve(Config { address, thread_count }, Arc::new(Mutex::new(handler))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("server: {error}");
            ExitCode::FAILURE
        }
    }
}
