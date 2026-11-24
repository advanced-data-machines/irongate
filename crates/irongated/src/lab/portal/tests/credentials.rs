//! Startup failure must not silently switch to the cleartext fixture.
use crate::lab::portal::{config::Settings, tls_config};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};
#[test]
fn bounded_private_credentials_reject_symlinks_permissions_and_invalid_files() {
    let root = std::env::temp_dir().join(format!("ig-tls-credentials-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let cert = root.join("cert.der");
    let key = root.join("key.der");
    fs::write(&cert, include_bytes!("fixtures/server.der")).unwrap();
    fs::write(&key, include_bytes!("fixtures/server.key.der")).unwrap();
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    let text = format!(
        "mode='https'\norigin='https://10.50.10.1:8443'\ncertificate_der='{}'\nprivate_key_der='{}'",
        cert.display(),
        key.display()
    );
    let config = root.join("portal.toml");
    fs::write(&config, &text).unwrap();
    let settings = Settings::load(&config).unwrap();
    assert!(settings.tls.is_some());
    assert!(settings.endpoint.secure);
    fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Settings::load(&config).is_err());
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    symlink(&key, root.join("linked.key")).unwrap();
    assert!(tls_config::read_bounded(&root.join("linked.key"), 16384, true).is_err());
    assert!(tls_config::read_bounded(&root, 16384, false).is_err());
    assert!(tls_config::read_bounded(Path::new("relative.key"), 16384, true).is_err());
    assert!(tls_config::read_bounded(&key, 1, true).is_err());
    fs::write(&key, []).unwrap();
    assert!(Settings::load(&config).is_err());
    fs::write(&config, [0xff]).unwrap();
    assert!(Settings::load(&config).is_err());
    fs::write(&config, "mode='https'\nmode='http-lab'").unwrap();
    assert!(Settings::load(&config).is_err());
    fs::remove_dir_all(root).unwrap();
}
