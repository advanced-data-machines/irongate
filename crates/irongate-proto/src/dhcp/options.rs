//! Validate the full TLV region once, then offer infallible borrowed iteration.

use super::{MAX_OPTION_BYTES, MAX_OPTIONS, MessageType};
use crate::{Error, Layer};

/// One option's code and borrowed value; PAD/END are omitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OptionView<'a> {
    /// Wire option code.
    pub code: u8,
    /// Value bytes without code or length.
    pub value: &'a [u8],
}

/// Iterator over an already validated bounded options region.
#[derive(Clone, Debug)]
pub struct Options<'a> {
    pub(super) bytes: &'a [u8],
}

impl<'a> Iterator for Options<'a> {
    type Item = OptionView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match *self.bytes.first()? {
                0 => self.bytes = &self.bytes[1..],
                255 => {
                    self.bytes = &[];
                    return None;
                }
                code => {
                    let end = 2 + usize::from(self.bytes[1]);
                    let value = &self.bytes[2..end];
                    self.bytes = &self.bytes[end..];
                    return Some(OptionView { code, value });
                }
            }
        }
    }
}

pub(super) fn validate(bytes: &[u8]) -> Result<MessageType, Error> {
    let layer = Layer::Dhcp;
    if bytes.len() > MAX_OPTION_BYTES {
        return Err(Error::LimitExceeded(layer));
    }
    let mut cursor = 0;
    let mut count = 0;
    let mut seen = 0u16;
    let mut message_type = None;
    while let Some(&code) = bytes.get(cursor) {
        cursor += 1;
        if code == 0 {
            continue;
        }
        if code == 255 {
            if bytes[cursor..].iter().any(|byte| *byte != 0) {
                return Err(Error::Malformed(layer));
            }
            return message_type.ok_or(Error::Malformed(layer));
        }
        count += 1;
        if count > MAX_OPTIONS {
            return Err(Error::LimitExceeded(layer));
        }
        let length = usize::from(*bytes.get(cursor).ok_or(Error::Truncated(layer))?);
        cursor += 1;
        let end = cursor + length;
        let value = bytes.get(cursor..end).ok_or(Error::Truncated(layer))?;
        cursor = end;
        if count == 1 && code != 53 {
            return Err(Error::Malformed(layer));
        }
        let bit = match code {
            53 => 1,
            50 => 2,
            54 => 4,
            57 => 8,
            61 => 16,
            55 => 32,
            _ => 0,
        };
        if seen & bit != 0 {
            return Err(Error::Malformed(layer));
        }
        seen |= bit;
        match code {
            53 if length == 1 => {
                message_type =
                    Some(MessageType::from_byte(value[0]).ok_or(Error::Unsupported(layer))?)
            }
            53 => return Err(Error::Malformed(layer)),
            50 | 54 if length != 4 => return Err(Error::Malformed(layer)),
            57 if length != 2 || u16::from_be_bytes([value[0], value[1]]) < 576 => {
                return Err(Error::Malformed(layer));
            }
            61 if length < 2 => return Err(Error::Malformed(layer)),
            55 if length == 0 => return Err(Error::Malformed(layer)),
            52 | 82 => return Err(Error::Unsupported(layer)),
            _ => {}
        }
    }
    Err(Error::Truncated(layer)) // A supported DHCP message must contain END.
}
