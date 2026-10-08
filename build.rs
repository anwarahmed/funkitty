//! Stamps the build with the git commit it came from, for `--version`, and lists the
//! voice clips in `voice/` so that a new thing for Pink Kitty to say is a new file.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() {
    let commit = git(&["rev-parse", "HEAD"]).unwrap_or_default();
    // A build with local changes is not any published version; it must never be
    // "updated" over, so it is marked and the update check leaves it alone.
    let dirty = git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty());
    println!("cargo:rustc-env=FUNKITTY_COMMIT={commit}{}", if dirty && !commit.is_empty() { "-dirty" } else { "" });
    // Committing changes no source file, so rerun on every build to keep the stamp
    // honest. The crate itself is only recompiled when the stamp actually changes.
    println!("cargo:rerun-if-changed=.force-rerun");

    // `CLIPS`: every `voice/*.wav` as (name without .wav, contents), by name.
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut names: Vec<String> = std::fs::read_dir(format!("{root}/voice"))
        .expect("the voice directory")
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.strip_suffix(".wav").map(String::from))
        .collect();
    names.sort();
    let entries: String = names.iter().map(|name| format!("    ({name:?}, include_bytes!({:?})),\n", format!("{root}/voice/{name}.wav"))).collect();
    let list = format!("/// The voice clips built into the program, in order of name.\nconst CLIPS: &[(&str, &[u8])] = &[\n{entries}];\n");
    std::fs::write(format!("{}/clips.rs", std::env::var("OUT_DIR").unwrap()), list).unwrap();
}
