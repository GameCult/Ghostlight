use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=GHOSTLIGHT_BUILD_COMMIT");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    let commit = std::env::var("GHOSTLIGHT_BUILD_COMMIT")
        .ok()
        .filter(|value| is_commit(value))
        .or_else(git_commit)
        .expect("GhostlightDungeon builds require an exact 40-hex source commit");
    println!("cargo:rustc-env=GHOSTLIGHT_BUILD_COMMIT={commit}");
}

fn git_commit() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir("../..")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    is_commit(&value).then_some(value)
}

fn is_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
