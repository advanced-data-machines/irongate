//! Strict bounded HTTP framing through httparse; no permissive second parser.

use super::bridge::Request;
pub(super) const MAX_REQUEST: usize = 8192;

#[allow(
    clippy::large_enum_variant,
    reason = "Bounded credentials remain inline and cleared on drop, without a heap indirection per parsed request."
)]
pub(super) enum Parsed {
    Incomplete,
    Ready {
        request: Request,
        html: bool,
        credentials: Option<crate::lab::radius::credentials::Credentials>,
    },
}

pub(super) fn parse(
    bytes: &[u8],
    entropy: [u8; 32],
    endpoint: &super::config::Endpoint,
) -> Result<Parsed, u16> {
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
    let headers = super::headers::check(parsed.headers, &endpoint.authority)?;
    let body_len = headers.content_length.unwrap_or(0);
    if body_len
        > if parsed.path == Some("/login") {
            768
        } else {
            128
        }
    {
        return Err(413);
    }
    let required = length.checked_add(body_len).ok_or(413u16)?;
    if bytes.len() < required {
        return Ok(Parsed::Incomplete);
    }
    if bytes.len() != required {
        return Err(400);
    }
    super::routes::parse(
        parsed.method,
        parsed.path,
        &bytes[length..],
        headers,
        entropy,
        endpoint,
    )
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
