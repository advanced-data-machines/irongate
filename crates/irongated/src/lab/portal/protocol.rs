//! Strict bounded HTTP framing through httparse; no permissive second parser.

use super::{bridge::Request, tokens::Action};
pub(super) const MAX_REQUEST: usize = 8192;
pub(super) const ORIGIN: &str = "http://10.50.10.1:8080";

pub(super) enum Parsed {
    Incomplete,
    Ready { request: Request, html: bool },
}

pub(super) fn parse(bytes: &[u8], entropy: [u8; 32]) -> Result<Parsed, u16> {
    if bytes.len() > MAX_REQUEST {
        return Err(413);
    }
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut parsed = httparse::Request::new(&mut headers);
    let length = match parsed.parse(bytes).map_err(|_| 400u16)? {
        httparse::Status::Partial => return Ok(Parsed::Incomplete),
        httparse::Status::Complete(length) => length,
    };
    if parsed.version != Some(1) {
        return Err(400);
    }
    let super::headers::Headers {
        accept_html,
        origin,
        content_length,
        content_type,
    } = super::headers::check(parsed.headers)?;
    let body_len = content_length.unwrap_or(0);
    if body_len > 128 {
        return Err(413);
    }
    let required = length.checked_add(body_len).ok_or(413u16)?;
    if bytes.len() < required {
        return Ok(Parsed::Incomplete);
    }
    if bytes.len() != required {
        return Err(400);
    }
    match (parsed.method, parsed.path) {
        (Some("GET"), Some("/" | "/api/action")) if body_len == 0 => Ok(Parsed::Ready {
            request: Request::View { entropy },
            html: parsed.path == Some("/"),
        }),
        (Some("GET"), Some("/status")) if body_len == 0 => Ok(Parsed::Ready {
            request: Request::Status,
            html: false,
        }),
        (Some("GET"), Some("/portal.js")) if body_len == 0 => Ok(Parsed::Ready {
            request: Request::Script,
            html: false,
        }),
        (Some("POST"), Some(path @ ("/consent" | "/logout"))) => {
            if origin != Some(ORIGIN) || content_type != Some("application/x-www-form-urlencoded") {
                return Err(403);
            }
            let body = &bytes[length..];
            let Some(hex) = body.strip_prefix(b"token=") else {
                return Err(400);
            };
            let token = decode(hex).ok_or(400u16)?;
            let action = if path == "/consent" {
                Action::Consent
            } else {
                Action::Logout
            };
            Ok(Parsed::Ready {
                request: Request::Act { action, token },
                html: accept_html,
            })
        }
        (Some("GET" | "POST"), _) => Err(404),
        _ => Err(405),
    }
}

pub(super) fn decode(value: &[u8]) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (out, pair) in bytes.iter_mut().zip(value.as_chunks::<2>().0) {
        let digit = |b: u8| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        };
        *out = (digit(pair[0])? << 4) | digit(pair[1])?;
    }
    Some(bytes)
}

pub(super) fn encode(token: [u8; 32]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in token {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 15) as usize] as char);
    }
    output
}
