//! Small HTTP route vocabulary after framing, headers and length are validated.
use super::{
    bridge::Request, config::Endpoint, headers::Headers, protocol::Parsed, tokens::Action,
};
pub(super) fn parse(
    method: Option<&str>,
    path: Option<&str>,
    body: &[u8],
    headers: Headers<'_>,
    entropy: [u8; 32],
    endpoint: &Endpoint,
) -> Result<Parsed, u16> {
    let Headers {
        accept_html,
        origin,
        content_type,
        ..
    } = headers;
    match (method, path) {
        (Some("GET"), Some("/" | "/api/action")) if body.is_empty() => Ok(Parsed::Ready {
            credentials: None,
            request: Request::View { entropy },
            html: path == Some("/"),
        }),
        (Some("GET"), Some("/status")) if body.is_empty() => Ok(Parsed::Ready {
            credentials: None,
            request: Request::Status,
            html: false,
        }),
        (Some("GET"), Some("/portal.js")) if body.is_empty() => Ok(Parsed::Ready {
            credentials: None,
            request: Request::Script,
            html: false,
        }),
        (Some("POST"), Some("/login")) => {
            if !endpoint.secure
                || origin != Some(endpoint.origin.as_str())
                || content_type != Some("application/x-www-form-urlencoded")
            {
                return Err(403);
            }
            let (token, credentials) = super::login_form::decode(body)?;
            Ok(Parsed::Ready {
                request: Request::Act {
                    action: Action::Login,
                    token,
                },
                html: accept_html,
                credentials: Some(credentials),
            })
        }
        (Some("POST"), Some(path @ ("/consent" | "/logout"))) => {
            if origin != Some(endpoint.origin.as_str())
                || content_type != Some("application/x-www-form-urlencoded")
            {
                return Err(403);
            }
            let Some(hex) = body.strip_prefix(b"token=") else {
                return Err(400);
            };
            let token = super::protocol::decode(hex).ok_or(400u16)?;
            let action = if path == "/consent" {
                Action::Consent
            } else {
                Action::Logout
            };
            Ok(Parsed::Ready {
                credentials: None,
                request: Request::Act { action, token },
                html: accept_html,
            })
        }
        (Some("GET" | "POST"), _) => Err(404),
        _ => Err(405),
    }
}
