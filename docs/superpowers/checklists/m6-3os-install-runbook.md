# M6 — 3-OS Install & Offline Runbook

Manual acceptance run on a **clean** machine per OS (no prior MindFlow data dir, no GPU required). Execute every step; tick the box; record latency and anomalies in Notes. This is the human half of the M6 release gate — it cannot be automated in CI.

**Build the bundle first (per OS):**

```bash
cd app
bun install
bun run tauri build
```

Artifacts:
- **Windows:** `app/src-tauri/target/release/bundle/nsis/*-setup.exe` (or `msi/`)
- **macOS:** `app/src-tauri/target/release/bundle/dmg/*.dmg` (or `macos/*.app`)
- **Linux:** `app/src-tauri/target/release/bundle/appimage/*.AppImage` (or `deb/`)

---

## Per-OS procedure

Repeat the whole list on **Windows**, **macOS**, and **Linux**.

### Install & onboard
- [ ] Install/launch the bundle on a clean account (no existing `com.mindflow.app` data dir).
- [ ] First-run onboarding appears with the new gold brand; complete: welcome → mic primer → (macOS) accessibility primer → model.
- [ ] A model is **auto-recommended** for this CPU; download completes and verifies.
- [ ] Onboarding finishes (try-it → features → main app).

### Dictate (network ON, baseline)
- [ ] Set Recording mode = **Hold**. In a third-party app (browser address bar / notes), hold the hotkey, speak a sentence, release.
- [ ] Punctuated, capitalized text appears in that app within ~1–2s. **Record latency:** ______
- [ ] No filler words ("um", "uh") in the output.

### Dictate (network OFF — the zero-network proof)
- [ ] **Cut the network** (airplane mode / disable all adapters). Confirm no connectivity.
- [ ] Dictate again into a third-party app → text still appears, same quality, similar latency.
- [ ] (Optional) Watch a network monitor during dictation → **no outbound connections**.

### Personalization (M4)
- [ ] Advanced → Dictionary: add a custom word (e.g., a name the model mishears). Dictate it → it is spelled correctly.
- [ ] Advanced → Replacements: add a rule (`from` → `to`). Dictate the `from` phrase → output shows `to`.
- [ ] Advanced → Snippets: add a spoken cue → expansion. Dictate the cue → expansion appears.

### Recording modes
- [ ] Set Recording mode = **Toggle**: tap hotkey to start, tap again to stop → transcribes.
- [ ] Set Recording mode = **Hands-free**: tap hotkey to start, press **Enter** to stop → transcribes.

### Brand surfaces
- [ ] Tray/menubar icon shows the gold brain mark; tray menu opens.
- [ ] During recording, the overlay shows the gold **Flow Mark island** (brain glyph + gold waveform + gold X), not the old pink pill.
- [ ] App icon in the taskbar/dock is the new dark-card brain (note: may require an OS icon-cache clear / reinstall on Windows).

### Update check (optional network)
- [ ] Re-enable network. Settings → confirm the update check works when triggered.
- [ ] Toggle `update_checks_enabled` **off** → no update network call occurs.

### Sign-off (this OS)
- OS / version: ____________________  CPU: ____________________
- Result: ☐ PASS  ☐ FAIL
- Notes / anomalies: ______________________________________________

---

## Summary

| OS | Pass | Tester | Date |
|----|------|--------|------|
| Windows | ☐ | | |
| macOS | ☐ | | |
| Linux | ☐ | | |
