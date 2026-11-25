use super::*;
#[test]
fn idle_interims_coalesce_and_stop_does_not_wait_for_failed_interim() {
    let (h, l) = handles();
    let mut q = Outbox::enabled([0; 16], 2);
    q.reserve(h, l, identity(), 0);
    q.activate(h, l, 0);
    let start = q.next(0).unwrap();
    q.complete(ack(start), 0);
    q.tick(2);
    let interim = q.next(2).unwrap();
    assert_eq!(interim.record.kind, Kind::Interim);
    assert_eq!(interim.record.octets, Octets::default());
    q.submitted(h, l, false, 7);
    q.tick(4);
    q.tick(20);
    q.complete(ack(interim), 20);
    let latest = q.next(20).unwrap();
    assert_eq!(latest.record.created, 20);
    assert_eq!(latest.record.octets.output, 7);
    q.stop(h, l, 21, 1);
    q.complete(
        Receipt {
            outcome: Outcome::Invalid,
            invalid: 2,
            ..ack(latest)
        },
        21,
    );
    assert!(!q.can_admit());
    assert_eq!(q.snapshot(22).invalid, 2);
    assert!(q.next(22).is_none());
    let stop = q.next(23).unwrap();
    assert_eq!(stop.record.kind, Kind::Stop);
    q.complete(ack(stop), 23);
    assert!(q.can_admit());
}
#[test]
fn stalled_dispatch_and_disconnection_are_visible_without_releasing_capacity() {
    let (h, l) = handles();
    let mut q = Outbox::enabled([0; 16], 2);
    q.reserve(h, l, identity(), 0);
    q.activate(h, l, 0);
    let start = q.next(0).unwrap();
    q.stop(h, l, 1, 1);
    q.tick(4);
    assert!(q.snapshot(4).stalled);
    assert!(q.next(40).is_none());
    assert!(!q.can_admit());
    q.disconnected();
    assert!(q.snapshot(4).unavailable);
    q.complete(ack(start), 5);
    assert!(!q.snapshot(5).stalled);
    let mut forged = ack(q.next(5).unwrap());
    forged.token.serial += 1;
    q.complete(forged, 5);
    assert_eq!(q.snapshot(5).retained, 1);
    assert_eq!(q.snapshot(5).stale, 1);
    assert_eq!(q.snapshot(6).json(6)["storage"], "memory");
    let mut disabled = Outbox::default();
    disabled.disconnected();
    assert!(disabled.can_admit());
    assert!(!disabled.snapshot(0).unavailable);
}
