//! End-to-end tests that spawn the real `rtz` binary, per the repo convention for
//! binaries (`CARGO_BIN_EXE_<name>`).

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_rtz");

#[test]
fn ned_tz_resolves_a_known_point() {
    let output = Command::new(BIN).args(["ned", "tz", "-87.62,41.88"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("America/Chicago"), "stdout was: {stdout}");
}

#[test]
fn osm_admin_resolves_a_known_point() {
    let output = Command::new(BIN).args(["osm", "admin", "30,30"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Name:"), "stdout was: {stdout}");
}

#[test]
fn malformed_lng_lat_exits_nonzero() {
    let output = Command::new(BIN).args(["ned", "tz", "not-a-coordinate"]).output().unwrap();
    assert!(!output.status.success());
}

#[test]
fn version_flag_prints_the_crate_version() {
    let output = Command::new(BIN).arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "stdout was: {stdout}");
}
