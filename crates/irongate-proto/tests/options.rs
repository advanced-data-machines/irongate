//! Bounded option scans and explicit exclusions for the DHCP pilot.

mod common;
use common::payload;
use irongate_proto::{
    Error, Layer,
    dhcp::{self, MAX_OPTION_BYTES, MAX_OPTIONS},
};

#[test]
fn unknown_options_are_borrowed_without_reordering_or_concatenating() {
    let bytes = payload(&[0, 53, 1, 1, 200, 2, 10, 20, 200, 1, 30, 255, 0]);
    let request = dhcp::parse(&bytes).unwrap();
    let options: Vec<_> = request.options().collect();
    assert_eq!(options.len(), 3);
    assert_eq!(options[1].value, &[10, 20]);
    assert_eq!(options[2].value, &[30]);
    assert_eq!(options[1].value.as_ptr(), bytes[246..].as_ptr());
}

#[test]
fn malformed_lengths_missing_end_and_duplicates_are_rejected() {
    for options in [
        &[53, 1][..],
        &[53, 1, 1],
        &[53, 1, 1, 200, 4, 1],
        &[53, 1, 1, 53, 1, 1, 255],
        &[53, 0, 255],
        &[53, 1, 1, 50, 3, 1, 2, 3, 255],
        &[53, 1, 1, 61, 1, 1, 255],
        &[53, 1, 1, 57, 1, 255, 255],
        &[53, 1, 1, 57, 2, 2, 63, 255],
        &[53, 1, 1, 255, 1],
        &[55, 1, 1, 53, 1, 1, 255],
    ] {
        assert!(dhcp::parse(&payload(options)).is_err(), "{options:?}");
    }
    assert!(dhcp::parse(&payload(&[53, 1, 1, 57, 2, 2, 64, 255])).is_ok());
}

#[test]
fn option_count_and_bytes_have_exact_capacity_boundaries() {
    let mut options = vec![53, 1, 1];
    for _ in 1..MAX_OPTIONS {
        options.extend_from_slice(&[200, 0]);
    }
    options.push(255);
    assert!(dhcp::parse(&payload(&options)).is_ok());
    options.splice(3..3, [200, 0]);
    assert_eq!(
        dhcp::parse(&payload(&options)).unwrap_err(),
        Error::LimitExceeded(Layer::Dhcp)
    );
    let mut options = vec![0; MAX_OPTION_BYTES];
    options[..3].copy_from_slice(&[53, 1, 1]);
    options[MAX_OPTION_BYTES - 1] = 255;
    assert!(dhcp::parse(&payload(&options)).is_ok());
    options.push(0);
    assert_eq!(
        dhcp::parse(&payload(&options)).unwrap_err(),
        Error::LimitExceeded(Layer::Dhcp)
    );
}

#[test]
fn relays_overload_and_nonclient_messages_are_explicitly_unsupported() {
    for options in [
        &[53, 1, 1, 52, 1, 1, 255][..],
        &[53, 1, 1, 82, 0, 255],
        &[53, 1, 9, 255],
        &[53, 1, 2, 255],
    ] {
        assert_eq!(
            dhcp::parse(&payload(options)).unwrap_err(),
            Error::Unsupported(Layer::Dhcp)
        );
    }
    for index in [3, 24] {
        let mut bytes = payload(&[53, 1, 1, 255]);
        bytes[index] = 1;
        assert_eq!(
            dhcp::parse(&bytes).unwrap_err(),
            Error::Unsupported(Layer::Dhcp)
        );
    }
}

#[test]
fn header_invariants_and_all_prefixes_are_checked() {
    let original = payload(&[53, 1, 1, 255]);
    for end in 0..original.len() {
        assert!(dhcp::parse(&original[..end]).is_err());
    }
    for index in [11, 28, 236] {
        let mut bytes = original.clone();
        bytes[index] ^= 1;
        assert_eq!(
            dhcp::parse(&bytes).unwrap_err(),
            Error::Malformed(Layer::Dhcp)
        );
    }
    for index in 0..original.len() {
        for value in [0, 1, 127, 255] {
            let mut bytes = original.clone();
            bytes[index] = value;
            if let Ok(request) = dhcp::parse(&bytes) {
                for option in request.options() {
                    std::hint::black_box(option);
                }
            }
        }
    }
}
