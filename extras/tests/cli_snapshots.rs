//! assert_cmd and insta CLI tests (unverified in M0; the std-only suite in dimvexa-cli covers
//! the same cases with checked-in snapshots).
use assert_cmd::Command;

#[test]
fn json_output_matches_snapshot() {
    let out = Command::cargo_bin("dimvexa")
        .unwrap()
        .args(["convert", "10", "m/s", "km/h", "--json"])
        .output()
        .unwrap();
    insta::assert_snapshot!(String::from_utf8(out.stdout).unwrap());
}
