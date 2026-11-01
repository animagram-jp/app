use alloc::{
    fmt::{self, Display, Formatter},
    string::{String, ToString},
    vec::Vec,
};
use core::{
    option::Option::{self, None, Some},
    primitive::{str, u8, u16, usize},
    result::Result::{self, Err, Ok},
    str::from_utf8,
};

use crate::WireError;

pub const HEADER_SIZE_MAX: usize = 8 * 1024;
pub const HEADER_COUNT_MAX: usize = 32;
pub const BODY_MAX: usize = 1024 * 1024;

const CRLF: &[u8] = b"\r\n";
const HEADER_END: &[u8] = b"\r\n\r\n";

#[derive(Debug, PartialEq)]
pub enum HttpError {
    RequestLine,
    Method,
    Target,
    Version,
    Header,
    HeaderTooLarge,
    BodyTooLarge,
    ContentLength,
    TransferEncoding,
}

impl Display for HttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for HttpError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        path.push(match self {
            HttpError::RequestLine => 1,
            HttpError::Method => 2,
            HttpError::Target => 3,
            HttpError::Version => 4,
            HttpError::Header => 5,
            HttpError::HeaderTooLarge => 6,
            HttpError::BodyTooLarge => 7,
            HttpError::ContentLength => 8,
            HttpError::TransferEncoding => 9,
        });
    }

    fn detail(&self) -> String {
        self.to_string()
    }

    fn status(&self) -> u16 {
        match self {
            HttpError::HeaderTooLarge => 431,
            HttpError::BodyTooLarge => 413,
            HttpError::TransferEncoding => 411,
            HttpError::Version => 505,
            _ => 400,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Delete = 1,
    Get,
    Post,
    Put,
}

impl Method {
    fn parse(token: &[u8]) -> Result<Self, HttpError> {
        match token {
            b"DELETE" => Ok(Method::Delete),
            b"GET" => Ok(Method::Get),
            b"POST" => Ok(Method::Post),
            b"PUT" => Ok(Method::Put),
            _ => Err(HttpError::Method),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Request {
    pub method:     Method,
    pub path:       String,
    pub query:      Option<String>,
    pub headers:    Vec<(String, String)>,
    pub body:       Vec<u8>,
    pub keep_alive: bool,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(field, _)| field == name).map(|(_, value)| value.as_str())
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.path.split('/').filter(|segment| !segment.is_empty())
    }
}

fn is_token(byte: u8) -> bool {
    matches!(byte, b'!' | b'#'..=b'\'' | b'*' | b'+' | b'-' | b'.' | b'0'..=b'9' | b'A'..=b'Z' | b'^'..=b'z' | b'|' | b'~')
}

fn is_target(byte: u8) -> bool {
    byte.is_ascii_graphic() && byte != b'#'
}

fn parse_target(bytes: &[u8]) -> Result<(String, Option<String>), HttpError> {
    if bytes.is_empty() || bytes[0] != b'/' || !bytes.iter().all(|byte| is_target(*byte)) {
        return Err(HttpError::Target);
    }
    let text = from_utf8(bytes).map_err(|_| HttpError::Target)?;
    let (path, query) = match text.split_once('?') {
        Some((path, query)) => (path, Some(query.to_string())),
        None => (text, None),
    };
    if path.split('/').any(|segment| segment == "." || segment == "..") {
        return Err(HttpError::Target);
    }
    Ok((path.to_string(), query))
}

fn parse_header(line: &[u8]) -> Result<(String, String), HttpError> {
    let colon = line.iter().position(|byte| *byte == b':').ok_or(HttpError::Header)?;
    let name = &line[..colon];
    if name.is_empty() || !name.iter().all(|byte| is_token(*byte)) {
        return Err(HttpError::Header);
    }
    let value = &line[colon + 1..];
    let value = value.trim_ascii();
    if !value.iter().all(|byte| *byte == b'\t' || (*byte >= 0x20 && *byte != 0x7f)) {
        return Err(HttpError::Header);
    }
    let name = from_utf8(name).map_err(|_| HttpError::Header)?.to_ascii_lowercase();
    let value = from_utf8(value).map_err(|_| HttpError::Header)?.to_string();
    Ok((name, value))
}

fn find(part: &[u8], whole: &[u8]) -> Option<usize> {
    if part.is_empty() || whole.len() < part.len() {
        return None;
    }
    whole.windows(part.len()).position(|window| window == part)
}

pub fn parse(buffer: &[u8]) -> Result<Option<(Request, usize)>, HttpError> {
    let header_end = match find(HEADER_END, &buffer[..buffer.len().min(HEADER_SIZE_MAX)]) {
        Some(at) => at,
        None if buffer.len() >= HEADER_SIZE_MAX => return Err(HttpError::HeaderTooLarge),
        None => return Ok(None),
    };
    let mut lines = buffer[..header_end]
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
    let request_line = lines.next().ok_or(HttpError::RequestLine)?;
    let mut parts = request_line.split(|byte| *byte == b' ');
    let method = Method::parse(parts.next().ok_or(HttpError::RequestLine)?)?;
    let (path, query) = parse_target(parts.next().ok_or(HttpError::RequestLine)?)?;
    let version = parts.next().ok_or(HttpError::RequestLine)?;
    if parts.next().is_some() {
        return Err(HttpError::RequestLine);
    }
    let mut keep_alive = match version {
        b"HTTP/1.1" => true,
        b"HTTP/1.0" => false,
        _ => return Err(HttpError::Version),
    };

    let mut headers = Vec::new();
    for line in lines {
        if headers.len() >= HEADER_COUNT_MAX {
            return Err(HttpError::HeaderTooLarge);
        }
        headers.push(parse_header(line)?);
    }

    let mut length = 0usize;
    for (name, value) in &headers {
        match name.as_str() {
            "content-length" => {
                length = value.parse().map_err(|_| HttpError::ContentLength)?;
                if length > BODY_MAX {
                    return Err(HttpError::BodyTooLarge);
                }
            }
            "transfer-encoding" => return Err(HttpError::TransferEncoding),
            "connection" => {
                keep_alive = match value.to_ascii_lowercase().as_str() {
                    "close" => false,
                    "keep-alive" => true,
                    _ => keep_alive,
                };
            }
            _ => {}
        }
    }

    let body_start = header_end + HEADER_END.len();
    let total = body_start + length;
    if buffer.len() < total {
        return Ok(None);
    }
    let body = buffer[body_start..total].to_vec();
    Ok(Some((Request { method, path, query, headers, body, keep_alive }, total)))
}

pub struct Response {
    pub status:  u16,
    pub headers: Vec<(String, String)>,
    pub body:    Vec<u8>,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Self { status, headers: Vec::new(), body: Vec::new() }
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn reason(status: u16) -> &'static str {
        match status {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            411 => "Length Required",
            413 => "Content Too Large",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            500 => "Internal Server Error",
            505 => "HTTP Version Not Supported",
            _ => "",
        }
    }

    pub fn to_bytes(&self, keep_alive: bool) -> Vec<u8> {
        let mut output = Vec::with_capacity(128 + self.body.len());
        output.extend_from_slice(b"HTTP/1.1 ");
        output.extend_from_slice(self.status.to_string().as_bytes());
        output.push(b' ');
        output.extend_from_slice(Self::reason(self.status).as_bytes());
        output.extend_from_slice(CRLF);
        output.extend_from_slice(b"Content-Length: ");
        output.extend_from_slice(self.body.len().to_string().as_bytes());
        output.extend_from_slice(CRLF);
        if !self.body.is_empty() {
            output.extend_from_slice(b"Content-Type: application/octet-stream\r\n");
        }
        output.extend_from_slice(b"Cache-Control: no-store\r\n");
        output.extend_from_slice(if keep_alive {
            b"Connection: keep-alive\r\n"
        } else {
            b"Connection: close\r\n"
        });
        for (name, value) in &self.headers {
            output.extend_from_slice(name.as_bytes());
            output.extend_from_slice(b": ");
            output.extend_from_slice(value.as_bytes());
            output.extend_from_slice(CRLF);
        }
        output.extend_from_slice(CRLF);
        output.extend_from_slice(&self.body);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_a_get_without_body() {
        let raw = b"GET /api/v1/health?x=1 HTTP/1.1\r\nHost: localhost\r\nX-Forwarded-For: 127.0.0.1\r\n\r\n";
        let (request, consumed) = parse(raw).unwrap().unwrap();
        assert_eq!(consumed, raw.len());
        assert_eq!(request.method, Method::Get);
        assert_eq!(request.path, "/api/v1/health");
        assert_eq!(request.query.as_deref(), Some("x=1"));
        assert_eq!(request.header("host"), Some("localhost"));
        assert!(request.keep_alive);
        assert_eq!(request.segments().collect::<Vec<_>>(), ["api", "v1", "health"]);
    }

    #[test]
    fn parse_leaves_bytes_after_the_body() {
        let raw = b"POST /a HTTP/1.0\r\nContent-Length: 3\r\n\r\nabcNEXT";
        let (request, consumed) = parse(raw).unwrap().unwrap();
        assert_eq!(request.body, b"abc");
        assert_eq!(consumed, raw.len() - 4);
        assert!(!request.keep_alive);
    }

    #[test]
    fn parse_waits_for_an_incomplete_request() {
        assert_eq!(parse(b"GET / HTTP/1.1\r\nHost: x\r\n").unwrap(), None);
        assert_eq!(parse(b"POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\nab").unwrap(), None);
    }

    #[test]
    fn parse_rejects_malformed_requests() {
        assert_eq!(parse(b"PATCH / HTTP/1.1\r\n\r\n").unwrap_err(), HttpError::Method);
        assert_eq!(parse(b"GET a HTTP/1.1\r\n\r\n").unwrap_err(), HttpError::Target);
        assert_eq!(parse(b"GET /../x HTTP/1.1\r\n\r\n").unwrap_err(), HttpError::Target);
        assert_eq!(parse(b"GET / HTTP/2\r\n\r\n").unwrap_err(), HttpError::Version);
        assert_eq!(parse(b"GET / HTTP/1.1\r\nBad\r\n\r\n").unwrap_err(), HttpError::Header);
        assert_eq!(
            parse(b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap_err(),
            HttpError::TransferEncoding
        );
        assert_eq!(
            parse(b"POST / HTTP/1.1\r\nContent-Length: 2000000\r\n\r\n").unwrap_err(),
            HttpError::BodyTooLarge
        );
        let mut oversized = b"GET / HTTP/1.1\r\n".to_vec();
        oversized.resize(HEADER_SIZE_MAX + 1, b'a');
        assert_eq!(parse(&oversized).unwrap_err(), HttpError::HeaderTooLarge);
    }

    #[test]
    fn a_response_serializes_its_status_headers_and_body() {
        let bytes =
            Response::new(404).body(b"xy".to_vec()).header("Set-Cookie", "a=b").to_bytes(false);
        let text = from_utf8(&bytes).unwrap();
        assert!(text.starts_with("HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\n"));
        assert!(text.contains("Content-Type: application/octet-stream\r\n"));
        assert!(text.contains("Connection: close\r\n"));
        assert!(text.contains("Set-Cookie: a=b\r\n"));
        assert!(text.ends_with("\r\n\r\nxy"));
        let empty = Response::new(204).to_bytes(true);
        assert!(!from_utf8(&empty).unwrap().contains("Content-Type"));
    }
}
