//! Startup must never open/configure networking on an ordinary invocation.

use std::process::Command;

#[test]
fn help_and_rejected_lab_invocations_do_not_enter_packet_dispatch() {
    let binary = env!("CARGO_BIN_EXE_irongated");
    for args in [vec![], vec!["--help"]] {
        let result = Command::new(binary).args(args).output().unwrap();
        assert!(result.status.success());
        assert!(
            String::from_utf8(result.stdout)
                .unwrap()
                .contains("owned QEMU guest harness")
        );
    }
    let result = Command::new(binary).arg("unknown").output().unwrap();
    assert_eq!(result.status.code(), Some(2));
    for run_id in ["invalid", "00000000000000000000000000000000"] {
        let result = Command::new(binary).args(["lab", run_id]).output().unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(
            String::from_utf8(result.stderr)
                .unwrap()
                .contains("lab failed")
        );
    }
}
