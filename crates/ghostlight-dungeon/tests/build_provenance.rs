use std::{path::Path, process::Command};

#[test]
fn embedded_commit_matches_checkout_head() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .expect("git must be available for the provenance regression test");
    assert!(output.status.success(), "git rev-parse HEAD failed");

    let checkout_head = String::from_utf8(output.stdout)
        .expect("git commit must be UTF-8")
        .trim()
        .to_owned();
    assert_eq!(
        env!("GHOSTLIGHT_BUILD_COMMIT"),
        checkout_head,
        "the binary embedded stale Git provenance"
    );
}
