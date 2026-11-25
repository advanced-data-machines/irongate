use super::state;
use crate::lab::{
    portal::{
        bridge::{Reply, Request},
        login_form,
        tokens::Action,
    },
    profile,
    radius::{Decision, Outcome},
};
use irongate_core::{lease, session};
fn begin(s: &mut crate::lab::state::State<'_>, entropy: u8) -> session::Attempt {
    let Reply::View(view) = s.portal_request(
        profile::CLIENT_IP.into(),
        Request::View {
            entropy: [entropy; 32],
        },
    ) else {
        panic!()
    };
    assert_eq!(view.action.unwrap().0, Action::Login);
    let Reply::Authenticate { attempt } = s.portal_request(
        profile::CLIENT_IP.into(),
        Request::Act {
            action: Action::Login,
            token: view.action.unwrap().1,
        },
    ) else {
        panic!()
    };
    attempt
}
fn decision(outcome: Outcome) -> Decision {
    Decision {
        outcome,
        elapsed_ms: 20,
        invalid: 0,
        sends: 1,
    }
}
#[test]
fn login_forms_are_bounded_and_reject_ambiguous_credentials_or_network_selection() {
    let token = "11".repeat(32);
    let body = format!("token={token}&username=lab-user&password=hello%2Bworld+space");
    let (_, credentials) = login_form::decode(body.as_bytes()).unwrap();
    assert_eq!(credentials.password(), b"hello+world space");
    for bad in [
        format!("{body}&network=2"),
        format!("{body}&password=second"),
        body.replace("%2B", "%QQ"),
        body.replace("lab-user", &"x".repeat(65)),
        body.replace("hello%2Bworld+space", &"x".repeat(129)),
        body.replace("lab-user", "%ff"),
    ] {
        assert!(login_form::decode(bad.as_bytes()).is_err());
    }
}
#[test]
fn accepted_results_apply_radius_policy_and_cancelled_results_cannot_grant() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    s.provider = session::Provider::Radius;
    let old = begin(&mut s, 1);
    s.control(b"logout");
    let current = begin(&mut s, 2);
    assert!(matches!(
        s.portal_request(
            profile::CLIENT_IP.into(),
            Request::RadiusResult {
                identity: None,
                attempt: old,
                decision: decision(Outcome::Accept { seconds: 5 })
            }
        ),
        Reply::Error(409, _)
    ));
    assert!(!s.allowed());
    assert!(matches!(
        s.portal_request(
            profile::CLIENT_IP.into(),
            Request::RadiusResult {
                identity: None,
                attempt: current,
                decision: decision(Outcome::Accept { seconds: 5 })
            }
        ),
        Reply::Redirect
    ));
    assert!(s.allowed());
    assert_eq!(
        s.sessions.view(s.active.unwrap()).unwrap().provider,
        Some(session::Provider::Radius)
    );
    assert_eq!(s.journal.used, 1);
    s.advance(5).unwrap();
    assert!(!s.allowed());
    assert_eq!(s.journal.used, 2);
}
#[test]
fn valid_reject_is_responsive_but_invalid_and_timed_out_attempts_are_not_healthy() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    s.provider = session::Provider::Radius;
    for (index, (outcome, code)) in [
        (Outcome::Reject, 401),
        (Outcome::Invalid, 502),
        (Outcome::Timeout, 503),
        (Outcome::Unavailable, 503),
        (Outcome::Unsupported, 422),
    ]
    .into_iter()
    .enumerate()
    {
        let attempt = begin(&mut s, index as u8 + 1);
        assert!(
            matches!(s.portal_request(profile::CLIENT_IP.into(),Request::RadiusResult{identity:None,attempt,decision:decision(outcome)}),Reply::Error(c,_)if c==code)
        );
        assert!(!s.allowed());
        let health = s.authentication.status(true, s.sessions.now());
        assert_eq!(
            health["health"],
            if matches!(outcome, Outcome::Reject | Outcome::Unsupported) {
                "responsive"
            } else {
                "unavailable"
            }
        );
    }
    assert_eq!(s.journal.used, 0);
}

#[test]
fn native_login_route_requires_https_exact_origin_and_bounded_credentials() {
    use crate::lab::portal::{
        config::Endpoint,
        protocol::{self, Parsed},
    };
    let endpoint = Endpoint::parse("https://10.50.10.1:8443").unwrap();
    let body = format!(
        "token={}&username=lab-user&password=lab-pass",
        "11".repeat(32)
    );
    let request = format!(
        "POST /login HTTP/1.1\r\nHost: 10.50.10.1:8443\r\nOrigin: https://10.50.10.1:8443\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    assert!(matches!(
        protocol::parse(request.as_bytes(), [1; 32], &endpoint),
        Ok(Parsed::Ready {
            request: Request::Act {
                action: Action::Login,
                ..
            },
            credentials: Some(_),
            ..
        })
    ));
    let wrong = request.replace("Origin: https:", "Origin: http:");
    assert!(matches!(
        protocol::parse(wrong.as_bytes(), [1; 32], &endpoint),
        Err(403)
    ));
    let http = request
        .replace("10.50.10.1:8443", "10.50.10.1:8080")
        .replace("https:", "http:");
    assert!(matches!(
        protocol::parse(http.as_bytes(), [1; 32], &Endpoint::http()),
        Err(403)
    ));
}
