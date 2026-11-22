//! Validated host requirements preserve network isolation context.

use irongate_config::check_str;
const VLANS: &str = include_str!("../../../examples/vlan-portals.toml");

#[test]
fn host_requirements_preserve_vlan_scope_and_exist_only_after_full_validation() {
    let report = check_str(VLANS, "fixture.toml");
    let requirements = report.host_requirements.unwrap();
    assert_eq!(requirements.upstream, "enp1s0");
    assert_eq!(requirements.networks.len(), 2);
    assert_eq!(requirements.networks[0].vlan_id, Some(10));
    assert_eq!(requirements.networks[1].vlan_id, Some(20));
    assert!(
        requirements
            .networks
            .iter()
            .all(|profile| profile.radius_authentication && profile.radius_accounting)
    );
    let invalid = VLANS.replacen("vlan_id = 20", "vlan_id = 10", 1);
    assert!(
        check_str(&invalid, "invalid.toml")
            .host_requirements
            .is_none()
    );
}
