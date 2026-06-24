// PLACEHOLDER GATE — this file is a scaffold installed in M1.
// It does NOT yet assert no network I/O because the dictation pipeline
// has not been built; that arrives in M2. M2–M6 MUST extend this test
// to drive the full dictation flow with networking disabled and assert
// zero outbound connections. Do not let later milestones ship without
// expanding it (see docs/superpowers/checklists/m1-injection-verification.md).

#[test]
fn placeholder_offline_gate_grows_in_m2() {
    // stand-in until M2 wires the pipeline
    let start = std::time::Instant::now();
    // Trivial CPU-only work as a placeholder for the pure local path.
    let mut acc = 0u64;
    for i in 0..1_000u64 {
        acc = acc.wrapping_add(i);
    }
    assert_eq!(acc, 499_500);
    assert!(
        start.elapsed().as_secs() < 2,
        "pure path must be fast and offline"
    );
}
