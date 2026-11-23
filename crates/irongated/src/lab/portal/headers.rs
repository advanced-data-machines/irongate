//! Single strict HTTP header policy shared by all portal routes.

pub(super) struct Headers<'a> {
    pub accept_html: bool,
    pub origin: Option<&'a str>,
    pub content_length: Option<usize>,
    pub content_type: Option<&'a str>,
}

pub(super) fn check<'a>(headers: &[httparse::Header<'a>]) -> Result<Headers<'a>, u16> {
    let mut accept_html = false;
    let mut host = None;
    let mut origin = None;
    let mut content_length = None;
    let mut content_type = None;
    for header in headers.iter() {
        let value = std::str::from_utf8(header.value).map_err(|_| 400u16)?;
        if header.name.eq_ignore_ascii_case("accept") {
            accept_html = value.contains("text/html");
        } else if header.name.eq_ignore_ascii_case("host") {
            if host.replace(value).is_some() {
                return Err(400);
            }
        } else if header.name.eq_ignore_ascii_case("origin") {
            if origin.replace(value).is_some() {
                return Err(400);
            }
        } else if header.name.eq_ignore_ascii_case("content-length") {
            if content_length.is_some()
                || value.is_empty()
                || !value.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(400);
            }
            content_length = Some(value.parse::<usize>().map_err(|_| 400u16)?);
        } else if header.name.eq_ignore_ascii_case("content-type") {
            if content_type.replace(value).is_some() {
                return Err(400);
            }
        } else if header.name.eq_ignore_ascii_case("transfer-encoding")
            || header.name.eq_ignore_ascii_case("upgrade")
        {
            return Err(400);
        }
    }
    if host != Some("10.50.10.1:8080") {
        return Err(403);
    }
    Ok(Headers {
        accept_html,
        origin,
        content_length,
        content_type,
    })
}
