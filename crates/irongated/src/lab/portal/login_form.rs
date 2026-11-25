//! Strict bounded form vocabulary; decoded credential buffers are cleared.
use crate::lab::radius::credentials::Credentials;
use zeroize::Zeroizing;
pub(super) fn decode(body: &[u8]) -> Result<([u8; 32], Credentials), u16> {
    let mut at = 0;
    while at < body.len() {
        if body[at] == b'%' {
            if at + 2 >= body.len() || !body[at + 1..at + 3].iter().all(u8::is_ascii_hexdigit) {
                return Err(400);
            }
            at += 2;
        }
        at += 1;
    }
    let (mut token, mut username, mut password) = (None, None, None);
    for (key, value) in form_urlencoded::parse(body) {
        let value = Zeroizing::new(value.into_owned());
        match key.as_ref() {
            "token" if token.is_none() => {
                token = Some(super::protocol::decode(value.as_bytes()).ok_or(400u16)?)
            }
            "username" if username.is_none() => username = Some(value),
            "password" if password.is_none() => password = Some(value),
            _ => return Err(400),
        }
    }
    let credentials = Credentials::new(
        username.as_deref().map(|s| s.as_str()).ok_or(400u16)?,
        password.as_deref().map(|s| s.as_str()).ok_or(400u16)?,
    )
    .ok_or(400u16)?;
    Ok((token.ok_or(400u16)?, credentials))
}
