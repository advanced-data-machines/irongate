//! Offline DHCP ownership transitions do not grant portal access.
mod common;
use common::*;
use irongate_core::lease::*;

#[test]
fn offer_request_ack_duplicate_and_renewal_have_explicit_deadlines() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut slots).unwrap();
    let client = key(1);
    advance(&mut table, 5);
    let offer = discover(&mut table, &client, 11);
    let view = table.view(offer.handle).unwrap();
    assert_eq!(view.phase, Phase::Offered);
    assert_eq!(view.expires_at, 15);
    advance(&mut table, 8);
    assert_eq!(discover(&mut table, &client, 11).handle, offer.handle);
    assert_eq!(table.view(offer.handle).unwrap().expires_at, 15); // Retransmit cannot extend hold.
    assert!(matches!(
        select(&mut table, &client, 11, offer.address),
        Outcome::Ack(_)
    ));
    assert_eq!(table.view(offer.handle).unwrap().expires_at, 128);
    advance(&mut table, 10);
    let Outcome::Ack(duplicate) = select(&mut table, &client, 11, offer.address) else {
        panic!()
    };
    assert_eq!(duplicate.lease_seconds, 118);
    assert_eq!(table.view(offer.handle).unwrap().expires_at, 128);
    advance(&mut table, 50);
    let outcome = table
        .apply(Command::Request {
            client: &client,
            xid: 12,
            mode: RequestMode::Renew {
                address: offer.address,
            },
        })
        .unwrap();
    assert!(matches!(outcome, Outcome::Ack(grant) if grant.handle == offer.handle));
    assert_eq!(table.view(offer.handle).unwrap().expires_at, 170);
    assert!(advance(&mut table, 128).is_empty()); // Old expiry cannot remove renewal.
    let events = advance(&mut table, 170);
    assert_eq!(
        events,
        [Event {
            handle: offer.handle,
            kind: EventKind::BindingExpired
        }]
    );
    assert!(table.view(offer.handle).is_none());
}

#[test]
fn selecting_requires_current_transaction_and_other_server_selection_is_silent() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut slots).unwrap();
    let client = key(1);
    let offer = discover(&mut table, &client, 11);
    discover(&mut table, &client, 12);
    assert_eq!(
        select(&mut table, &client, 11, offer.address),
        Outcome::Nak(NakReason::NoOffer)
    );
    let other = |xid| Command::Request {
        client: &client,
        xid,
        mode: RequestMode::Selecting {
            server: [10, 0, 0, 2],
            address: [10, 0, 0, 200],
        },
    };
    assert_eq!(
        table.apply(other(11)).unwrap(),
        Outcome::Ignore(IgnoreReason::OtherServer)
    );
    assert!(table.view(offer.handle).is_some());
    assert_eq!(
        table.apply(other(12)).unwrap(),
        Outcome::Ignore(IgnoreReason::OtherServer)
    );
    assert!(table.view(offer.handle).is_none());
}

#[test]
fn reboot_requests_ack_known_bindings_and_stay_silent_for_unknown_clients() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut slots).unwrap();
    let known = key(1);
    let unknown = key(2);
    let grant = bind(&mut table, &known, 1);
    let request = |client, address| Command::Request {
        client,
        xid: 2,
        mode: RequestMode::InitReboot { address },
    };
    assert!(matches!(
        table.apply(request(&known, grant.address)).unwrap(),
        Outcome::Ack(_)
    ));
    assert_eq!(
        table.apply(request(&unknown, grant.address)).unwrap(),
        Outcome::Ignore(IgnoreReason::NoBinding)
    );
    assert_eq!(
        table.apply(request(&unknown, [10, 1, 0, 1])).unwrap(),
        Outcome::Nak(NakReason::WrongAddress)
    );
    assert_eq!(
        table.apply(request(&known, [10, 0, 0, 99])).unwrap(),
        Outcome::Nak(NakReason::BindingMismatch)
    );
}

#[test]
fn rediscovery_keeps_existing_binding_until_successful_request_or_expiry() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut slots).unwrap();
    let client = key(1);
    let bound = bind(&mut table, &client, 1);
    advance(&mut table, 20);
    let rediscovered = discover(&mut table, &client, 2);
    assert_eq!(rediscovered.handle, bound.handle);
    assert_eq!(rediscovered.lease_seconds, 100); // DISCOVER itself does not extend a binding.
    assert_eq!(table.view(bound.handle).unwrap().phase, Phase::Bound);
    assert_eq!(
        advance(&mut table, 30),
        [Event {
            handle: bound.handle,
            kind: EventKind::PendingOfferExpired
        }]
    );
    assert_eq!(table.view(bound.handle).unwrap().expires_at, 120);
    assert_eq!(
        select(&mut table, &client, 2, bound.address),
        Outcome::Nak(NakReason::NoOffer)
    );
}

#[test]
fn only_owner_can_release_or_decline_and_decline_quarantine_is_timed() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut slots).unwrap();
    let client = key(1);
    let stranger = key(2);
    let grant = bind(&mut table, &client, 1);
    let release = |client| Command::Release {
        client,
        address: grant.address,
        server: config().server,
    };
    assert_eq!(
        table.apply(release(&stranger)).unwrap(),
        Outcome::Ignore(IgnoreReason::NotOwner)
    );
    assert_eq!(
        table.apply(release(&client)).unwrap(),
        Outcome::Released(grant.handle)
    );
    assert!(table.view(grant.handle).is_none());
    let offer = discover(&mut table, &client, 2);
    assert_eq!(
        table
            .apply(Command::Decline {
                client: &client,
                address: offer.address,
                server: config().server
            })
            .unwrap(),
        Outcome::Declined(offer.handle)
    );
    let view = table.view(offer.handle).unwrap();
    assert_eq!(view.phase, Phase::Declined);
    assert!(view.client.is_none());
    assert!(advance(&mut table, 29).is_empty());
    assert_eq!(
        advance(&mut table, 30),
        [Event {
            handle: offer.handle,
            kind: EventKind::QuarantineEnded
        }]
    );
    assert!(table.view(offer.handle).is_none());
}
