//! Exercise the real owner/worker boundary without binding a host TCP service.
use super::state;
use crate::lab::{
    portal::{
        bridge::{Bridge, Envelope, Reply, Request},
        page,
        tokens::Action,
        writer,
    },
    profile,
};
use irongate_core::{lease, session};
use std::{
    fs::OpenOptions,
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn owner_delivers_scoped_views_and_writer_acknowledges_ordered_grant_and_stop() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut state = state(&mut leases, &mut sessions);
    let (bridge, worker) = Bridge::pair().unwrap();
    let path = std::env::temp_dir().join(format!("ig-bridge-{}", std::process::id()));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let crate::lab::portal::bridge::WorkerBridge {
        sender,
        records,
        record_acks,
        wake,
        ..
    } = worker;
    let thread = std::thread::spawn(move || {
        writer::run(file, records, record_acks, wake, "test-run".to_owned())
    });
    let (reply, received) = mpsc::sync_channel(1);
    sender
        .try_send(Envelope {
            peer: profile::CLIENT_IP.into(),
            request: Request::View { entropy: [7; 32] },
            reply,
        })
        .unwrap_or_else(|_| panic!());
    bridge.budget(&mut state);
    let Reply::View(view) = received.try_recv().unwrap() else {
        panic!()
    };
    let html = String::from_utf8(page::response(Reply::View(view), true)).unwrap();
    assert!(html.contains("Accept and connect"));
    assert!(html.contains("Referrer-Policy: same-origin"));
    assert!(!page::status(view).to_string().contains("password"));
    let token = view.action.unwrap().1;
    assert!(matches!(
        state.portal_request(
            profile::CLIENT_IP.into(),
            Request::Act {
                action: Action::Consent,
                token
            }
        ),
        Reply::Redirect
    ));
    let Reply::View(view) = state.portal_request(
        profile::CLIENT_IP.into(),
        Request::View { entropy: [8; 32] },
    ) else {
        panic!()
    };
    assert!(
        String::from_utf8(page::response(Reply::View(view), true))
            .unwrap()
            .contains("Disconnect")
    );
    state.portal_request(
        profile::CLIENT_IP.into(),
        Request::Act {
            action: Action::Logout,
            token: view.action.unwrap().1,
        },
    );
    let limit = Instant::now() + Duration::from_secs(2);
    while state.journal.durable < 2 {
        assert!(Instant::now() < limit, "writer acknowledgement timed out");
        bridge.budget(&mut state);
        std::thread::yield_now();
    }
    let rows: Vec<serde_json::Value> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["kind"], "Granted");
    assert_eq!(rows[1]["kind"], "Revoked");
    assert_eq!(rows[0]["run_id"], "test-run");
    assert!(
        String::from_utf8(page::response(Reply::Script, false))
            .unwrap()
            .contains("/status")
    );
    assert!(
        String::from_utf8(page::response(Reply::Error(503, "Unavailable"), true))
            .unwrap()
            .contains("Return to the portal")
    );
    drop(bridge);
    thread.join().unwrap();
    std::fs::remove_file(path).unwrap();
}
