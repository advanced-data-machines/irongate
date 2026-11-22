//! Offline admission walkthrough with a simulated policy worker.

#[path = "../tests/common/mod.rs"]
mod common;
#[path = "../tests/support/session.rs"]
mod support;

use irongate_core::{
    lease,
    session::{self, Phase},
};

fn main() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [session::Slot::EMPTY; 2];
    let mut sessions = session::Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    println!(
        "DHCP binding attached: {:?}; allowed={}",
        sessions.view(handle).unwrap().phase,
        sessions.allows(&leases, handle)
    );
    let attempt = sessions.begin_consent(&leases, handle).unwrap();
    println!(
        "Consent opened: {:?}; allowed=false",
        sessions.view(handle).unwrap().phase
    );
    sessions.consent(&leases, attempt, 30).unwrap();
    println!(
        "Consent accepted: {:?}; allowed=false",
        sessions.view(handle).unwrap().phase
    );
    let apply = sessions.take_effect(&leases).unwrap().unwrap();
    println!("Simulated worker applying {:?}; allowed=false", apply.kind);
    sessions.complete(&leases, apply.token, true).unwrap();
    assert!(sessions.allows(&leases, handle));
    println!("Application acknowledged: Authorized; allowed=true");
    sessions.logout(handle).unwrap();
    println!(
        "Logout: {:?}; allowed={}",
        sessions.view(handle).unwrap().phase,
        sessions.allows(&leases, handle)
    );
    let removal = sessions.take_effect(&leases).unwrap().unwrap();
    sessions.complete(&leases, removal.token, false).unwrap();
    println!("Simulated removal failure: cleanup retained; allowed=false");
    support::remove(&mut sessions, &leases);
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Revoked);
    println!("Removal acknowledged: Revoked; storage can now be freed");
    support::authorize(&mut sessions, &leases, handle, 5);
    support::tick(&mut leases, &mut sessions, 5);
    println!(
        "New admission expired: {:?}; allowed={}",
        sessions.view(handle).unwrap().phase,
        sessions.allows(&leases, handle)
    );
    support::remove(&mut sessions, &leases);
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Expired);
    sessions.forget(handle).unwrap();
    println!("Expiry cleanup confirmed: Expired; old handle no longer resolves");
    println!(
        "Offline only: simulated effects, no portal, sockets, firewall or host-network changes."
    );
}
