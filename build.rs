use std::fs;
use std::process::Command;

fn main() {
    emit_git_rerun_hints();

    let build_date =
        command_output("date", &["-u", "+%Y-%m-%d"]).unwrap_or_else(|| "unknown-date".to_string());
    let git_hash = command_output("git", &["rev-parse", "--short", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_IGNORE_BUILD_DATE={build_date}");
    println!("cargo:rustc-env=GIT_IGNORE_GIT_HASH={git_hash}");
}

fn emit_git_rerun_hints() {
    println!("cargo:rerun-if-changed=.git/HEAD");

    let Ok(head) = fs::read_to_string(".git/HEAD") else {
        return;
    };
    let Some(ref_path) = head.trim().strip_prefix("ref: ") else {
        return;
    };

    println!("cargo:rerun-if-changed=.git/{ref_path}");
}

fn command_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}
