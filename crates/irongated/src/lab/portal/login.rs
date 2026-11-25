//! Backend work stays on the service worker; only typed completion reaches owner.
use super::{
    bridge::{Envelope, Reply, Request},
    config::Settings,
};
use crate::lab::radius::credentials::Credentials;
use std::{
    net::Ipv4Addr,
    sync::mpsc::{self, SyncSender},
    time::Duration,
};
pub(super) fn ask(
    sender: &SyncSender<Envelope>,
    peer: Ipv4Addr,
    request: Request,
    notify: impl Fn(),
) -> Result<Reply, u16> {
    let (reply, receiver) = mpsc::sync_channel(1);
    sender
        .try_send(Envelope {
            peer,
            request,
            reply,
        })
        .map_err(|_| 503u16)?;
    notify();
    receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|_| 503)
}
pub(super) fn complete(
    reply: Reply,
    credentials: Option<Credentials>,
    settings: &Settings,
    peer: Ipv4Addr,
    sender: &SyncSender<Envelope>,
    notify: impl Fn(),
) -> Result<Reply, u16> {
    let Reply::Authenticate { attempt } = reply else {
        return Ok(reply);
    };
    let config = settings.radius.as_ref().ok_or(403u16)?;
    let credentials = credentials.ok_or(400u16)?;
    let decision = crate::lab::radius::client::authenticate(config, &credentials);
    ask(
        sender,
        peer,
        Request::RadiusResult {
            attempt,
            decision,
            identity: if matches!(decision.outcome, crate::lab::radius::Outcome::Accept { .. }) {
                crate::lab::accounting::UserName::new(credentials.user())
            } else {
                None
            },
        },
        notify,
    )
}
