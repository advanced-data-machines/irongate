use super::*;
#[test]
fn lifecycle_reserves_stop_orders_delivery_and_fences_reused_session() {
    let (h, l) = handles();
    let mut q = Outbox::enabled([1; 16], 2);
    assert!(q.reserve(h, l, identity(), 0));
    assert!(q.next(0).is_none());
    q.activate(h, l, 0);
    q.submitted(h, l, true, (1u64 << 32) + 17);
    q.submitted(h, l, false, 43);
    let start = q.next(0).unwrap();
    assert_eq!(start.record.kind, Kind::Start);
    q.stop(h, l, 3, 1);
    assert!(q.next(3).is_none());
    q.complete(ack(start), 3);
    let stop = q.next(3).unwrap();
    assert_eq!(stop.record.kind, Kind::Stop);
    assert_eq!(stop.record.octets.input, (1u64 << 32) + 17);
    assert_eq!(stop.record.octets.output, 43);
    // The same handle may authenticate again while its older Stop is pending.
    assert!(q.reserve(h, l, identity(), 3));
    q.activate(h, l, 3);
    q.submitted(h, l, true, 9);
    q.complete(ack(stop), 3);
    q.complete(ack(stop), 3);
    assert_eq!(q.snapshot(3).retained, 1);
    assert_eq!(q.snapshot(3).stale, 1);
    let new = q.next(3).unwrap();
    assert_ne!(start.record.session_id, new.record.session_id);
    q.complete(ack(new), 3);
    q.stop(h, l, 4, 5);
    let stop = q.next(4).unwrap();
    assert_eq!(stop.record.octets.input, 9);
    assert_eq!(stop.record.cause, 5);
    q.complete(ack(stop), 4);
    assert_eq!(q.snapshot(4).retained, 0);
}
#[test]
fn finite_capacity_and_failed_start_never_evict_stop() {
    let (h, l) = handles();
    let mut q = Outbox::enabled([0; 16], 2);
    for n in 0..4 {
        assert!(q.reserve(h, l, identity(), n));
        q.activate(h, l, n);
        q.stop(h, l, n + 1, 1);
    }
    assert!(!q.can_admit());
    assert!(!q.reserve(h, l, identity(), 5));
    let start = q.next(5).unwrap();
    q.complete(
        Receipt {
            outcome: Outcome::Timeout,
            ..ack(start)
        },
        6,
    );
    let other = q.next(7).unwrap();
    assert_ne!(other.record.session_id, start.record.session_id);
    q.complete(ack(other), 7);
    let retry = q.next(8).unwrap();
    assert_eq!(retry.record.session_id, start.record.session_id);
    assert_eq!(retry.record.kind, Kind::Start);
    assert_eq!(retry.delay, 8);
    q.complete(ack(retry), 8);
    let stop = q.next(8).unwrap();
    assert_eq!(stop.record.kind, Kind::Stop);
    q.complete(ack(stop), 8);
    assert!(q.can_admit());
}
#[test]
fn abandoned_reservation_counter_saturation_and_boot_identity() {
    let (h, l) = handles();
    let mut q = Outbox::enabled([1; 16], 2);
    assert!(q.reserve(h, l, identity(), 0));
    q.cancel();
    assert_eq!(q.snapshot(0).retained, 0);
    assert!(q.reserve(h, l, identity(), 0));
    q.activate(h, l, 0);
    q.submitted(h, l, true, u64::MAX);
    q.submitted(h, l, true, 1);
    let current = q.snapshot(0).current.unwrap();
    assert_eq!(current.input, u64::MAX);
    assert!(current.overflow);
    let first = q.next(0).unwrap();
    let mut other = Outbox::enabled([2; 16], 2);
    assert!(other.reserve(h, l, identity(), 0));
    other.activate(h, l, 0);
    assert_ne!(
        first.record.session_id,
        other.next(0).unwrap().record.session_id
    );
    q.serial = u64::MAX;
    q.stop(h, l, 1, 1);
    assert!(!q.can_admit());
    assert!(UserName::new(b"").is_none());
    assert!(UserName::new(&[b'x'; 65]).is_none());
    assert!(UserName::new(b"a\nb").is_none());
}
