use std::{env, path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=GHOSTLIGHT_BUILD_COMMIT");
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").expect("Cargo must supply CARGO_MANIFEST_DIR");
    let repo_root = Path::new(&manifest_dir).join("../..");
    emit_git_rerun_paths(&repo_root);
    // Release tooling injects the clean-tree commit. Git discovery is only a
    // convenience for local builds. The exact ref and reflog dependencies above
    // keep that convenience from silently publishing stale provenance.
    let commit = env::var("GHOSTLIGHT_BUILD_COMMIT")
        .ok()
        .filter(|value| is_commit(value))
        .or_else(|| git_commit(&repo_root))
        .expect("GhostlightDungeon builds require an exact 40-hex source commit");
    println!("cargo:rustc-env=GHOSTLIGHT_BUILD_COMMIT={commit}");
}

fn emit_git_rerun_paths(repo_root: &Path) {
    for git_path in ["HEAD", "logs/HEAD", "packed-refs"] {
        if let Some(path) = git_output(repo_root, &["rev-parse", "--git-path", git_path]) {
            emit_rerun_path(repo_root, &path);
        }
    }

    if let Some(reference) = git_output(repo_root, &["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git_output(repo_root, &["rev-parse", "--git-path", &reference])
    {
        emit_rerun_path(repo_root, &path);
    }
}

fn emit_rerun_path(repo_root: &Path, git_path: &str) {
    let path = Path::new(git_path);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    };
    println!("cargo:rerun-if-changed={}", resolved.display());
}

fn git_commit(repo_root: &Path) -> Option<String> {
    git_output(repo_root, &["rev-parse", "HEAD"]).filter(|value| is_commit(value))
}

fn git_output(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn is_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
