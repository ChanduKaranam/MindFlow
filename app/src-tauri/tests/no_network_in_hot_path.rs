//! Guard: the dictation hot-path must never make network calls.
//!
//! This is the always-on regression gate behind MindFlow's "fully local,
//! zero-network dictation" guarantee. It scans the source of every hot-path
//! module and fails if a network symbol (reqwest/hyper/ureq/std::net/sockets)
//! appears. Opt-in network features (model download, app update check,
//! post-processing LLM) live OUTSIDE this set and are intentionally excluded.
//!
//! If you legitimately add a network feature, it must NOT be in the dictation
//! path — put it in its own module and leave it out of `HOT_PATH`.

use std::fs;
use std::path::{Path, PathBuf};

/// Hot-path modules/files, relative to `src-tauri/src/`. MUST match the
/// hot-path list in docs/superpowers/audits/2026-06-27-m6-zero-network-audit.md.
const HOT_PATH: &[&str] = &[
    "audio_toolkit",
    "format",
    "replace",
    "managers/transcription.rs",
    "managers/audio.rs",
    "transcription_coordinator.rs",
    "actions.rs",
    "signal_handle.rs",
    "shortcut",
    "clipboard.rs",
    "input.rs",
];

/// Network symbols that must never appear in the hot-path. Crate/std-level
/// signals (an actual network call needs one of these), not raw URLs — URLs
/// show up in doc comments and would be noisy false positives.
const FORBIDDEN: &[&str] = &[
    "reqwest",
    "hyper::",
    "ureq",
    "std::net",
    "TcpStream",
    "TcpListener",
    "UdpSocket",
];

fn collect_rs(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().map_or(false, |e| e == "rs") {
            out.push(path.to_path_buf());
        }
        return;
    }
    for entry in fs::read_dir(path).expect("read_dir hot-path") {
        collect_rs(&entry.expect("dir entry").path(), out);
    }
}

/// Skip comment lines so a `// see https://… reqwest docs` comment does not
/// trip the guard.
fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with('*') || t.starts_with("/*")
}

#[test]
fn dictation_hot_path_has_no_network_symbols() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    for entry in HOT_PATH {
        let p = src.join(entry);
        assert!(
            p.exists(),
            "hot-path entry missing — update HOT_PATH and the audit doc: {}",
            p.display()
        );
        collect_rs(&p, &mut files);
    }
    assert!(!files.is_empty(), "no hot-path source files collected");

    let mut violations = Vec::new();
    for f in &files {
        let content = fs::read_to_string(f).expect("read hot-path file");
        for (i, line) in content.lines().enumerate() {
            if is_comment(line) {
                continue;
            }
            for sym in FORBIDDEN {
                if line.contains(sym) {
                    violations.push(format!(
                        "{}:{}: forbidden network symbol {:?}\n      {}",
                        f.display(),
                        i + 1,
                        sym,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Network symbols found in the dictation hot-path — the 'fully local' \
         guarantee is broken. Move network code out of the hot-path:\n{}",
        violations.join("\n")
    );
}
