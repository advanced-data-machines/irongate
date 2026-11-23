use super::state;
use crate::lab::{
    portal::{
        bridge::{Reply, Request},
        tokens::Action,
        writer,
    },
    profile,
};
use irongate_core::{lease, session};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    sync::atomic::{AtomicU64, Ordering},
};

#[test]
fn failed_and_stale_writer_acknowledgements_never_discard_retained_records() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    let Reply::View(view) = s.portal_request(
        profile::CLIENT_IP.into(),
        Request::View { entropy: [1; 32] },
    ) else {
        panic!()
    };
    let (_, token, _) = view.action.unwrap();
    s.portal_request(
        profile::CLIENT_IP.into(),
        Request::Act {
            action: Action::Consent,
            token,
        },
    );
    let record = s.journal.next(0).unwrap();
    s.journal.complete(99, true, 0);
    assert_eq!(s.journal.in_flight, Some(record.id));
    s.journal.tick(2);
    assert!(s.journal.stalled);
    assert!(s.journal.unavailable);
    assert!(s.journal.next(2).is_none()); // Unknown write outcome cannot be reordered.
    assert_eq!(s.journal.pending_age(2), 2);
    s.journal.complete(record.id, false, 2);
    assert_eq!(s.journal.durable, 0);
    assert!(s.journal.unavailable);
    // Storage failure must not delay revocation, but must reject a new grant.
    s.control(b"logout");
    assert!(!s.allowed());
    let Reply::View(view) = s.portal_request(
        profile::CLIENT_IP.into(),
        Request::View { entropy: [2; 32] },
    ) else {
        panic!()
    };
    assert!(matches!(
        s.portal_request(
            profile::CLIENT_IP.into(),
            Request::Act {
                action: Action::Consent,
                token: view.action.unwrap().1
            }
        ),
        Reply::Error(503, _)
    ));
    assert_eq!(s.journal.used, 2);
    assert!(s.journal.next(0).is_none());
    assert!(s.journal.next(2).is_none());
    let retry = s.journal.next(3).unwrap();
    assert_eq!(retry.id, record.id);
    s.journal.complete(retry.id, true, 3);
    assert_eq!(s.journal.durable, 1);
    assert!(!s.journal.unavailable);
    s.control(b"logout");
    assert_eq!(s.journal.used, 2);
    assert_eq!(s.journal.reserved, None);
}

#[test]
fn writer_ack_prefix_is_synced_and_partial_tail_is_removed_before_retry() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "ig-portal-journal-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let mut committed = 0;
    writer::append(&mut file, &mut committed, b"first\n").unwrap();
    file.write_all(b"partial-unacknowledged-tail").unwrap();
    writer::append(&mut file, &mut committed, b"second\n").unwrap();
    assert_eq!(committed, 13);
    assert!(writer::append(&mut file, &mut committed, &vec![0; 65536]).is_err());
    let mut contents = String::new();
    std::fs::File::open(&path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents, "first\nsecond\n");
    drop(file);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn journal_open_preserves_previous_evidence_and_private_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let directory = std::env::temp_dir().join(format!("ig-journal-open-{}", std::process::id()));
    let file = writer::open_at(&directory).unwrap();
    assert_eq!(
        std::fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
    assert!(writer::open_at(&directory).is_err());
    drop(file);
    std::fs::remove_file(directory.join("local-records.jsonl")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
