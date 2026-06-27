# M6 — v1.0 Release Gate

The final checklist before tagging and publishing **MindFlow v1.0.0**. Do not tag until every box is ticked.

## Automated checks
- [ ] `cd app && bun run build` — green (tsc + vite).
- [ ] `cd app && bun run lint` — clean.
- [ ] `cd app/src-tauri && cargo test` — green, including:
  - [ ] `no_network_in_hot_path::dictation_hot_path_has_no_network_symbols`
  - [ ] `zero_network::transcribes_offline_on_cpu` (skips without model)
  - [ ] `settings::tests::*` (incl. `default_settings_are_deterministic`), `replace::replacements::tests::*`, `format::spoken_commands::tests::*`, `recording_mode_tests::*`
- [ ] CI green on all three OSes for the release commit.

## Offline proof (maintainer, once)
- [ ] `MINDFLOW_TEST_MODEL=<path-to-moonshine-base> cargo test --test zero_network -- --nocapture` passes (real offline transcription + dictionary).

## Documentation gates
- [ ] Zero-network audit (`docs/superpowers/audits/2026-06-27-m6-zero-network-audit.md`) reviewed — no open items.
- [ ] DoD checklist (`docs/superpowers/checklists/m6-definition-of-done.md`) — all items checked.
- [ ] 3-OS install runbook (`docs/superpowers/checklists/m6-3os-install-runbook.md`) — PASS on Windows, macOS, and Linux.

## Versioning & metadata
- [ ] Version bumped to `1.0.0` and consistent across:
  - `app/src-tauri/tauri.conf.json` (`version`)
  - `app/src-tauri/Cargo.toml` (`package.version`)
  - `app/package.json` (`version`)
- [ ] Updater `pubkey` present and `endpoints` point at the MindFlow releases (sanity check; unchanged).
- [ ] `productName` / bundle identifier are MindFlow / `com.mindflow.app` (already set in M5).

## Artifacts (automated)
The `.github/workflows/release.yml` workflow builds installers on a 3-OS matrix
and uploads them to a **draft** GitHub Release when a `v*` tag is pushed:
- [ ] `git tag v1.0.0 && git push origin v1.0.0` → workflow runs green on all 3 OSes.
- [ ] Draft release contains: Windows `.exe` (NSIS) + `.msi`, macOS `.dmg` (Apple Silicon), Linux `.AppImage`.
- [ ] Each installer launch-tested (covered by the 3-OS runbook).

> Builds are **unsigned** (the upstream Windows signCommand was removed and
> `createUpdaterArtifacts` is off). Code signing + auto-update are a future
> setup needing your own certs/keys.

## Release
- [ ] Changelog / release notes drafted (highlights: fully-local CPU dictation, hands-free Enter-to-stop, dictionary/snippets, MindFlow identity).
- [ ] Review the draft release assets, then **Publish** it → others can download the installers.

---

**v1 is "done" when:** a user on a clean, GPU-less Win/macOS/Linux machine installs MindFlow, onboards, and dictates punctuated text into any app **fully offline**, with dictionary + snippets working — and the `no_network_in_hot_path` guard keeps it that way.
