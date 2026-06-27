# Task 13 Report — TryItNowStep

## Event/Source Decision

### Text arrival
Production path: `actions.rs` → `utils::paste()` → clipboard paste to OS-focused element. The component auto-focuses its `<textarea>` on mount so the paste lands there. Detection is `onChange` on the textarea (empty → non-empty). This is simpler than listening to `historyUpdatePayload` and correctly handles the case where the user types the sample manually during a demo session.

The `historyUpdatePayload` typed event was considered but rejected: it would require a second listener, and the `entry.transcription_text` arrives a few milliseconds before the paste completes, introducing a subtle timing race.

### Timer start signal
The backend emits `"show-overlay"` with payload `"recording"` the moment audio capture begins (`overlay.rs:339 → show_recording_overlay → emit("show-overlay", "recording")`). This is the earliest reliable frontend signal — earlier than the textarea receiving text by the full audio + transcription duration. The component subscribes to this event and sets `recordingStartedAt.current = Date.now()` on arrival.

Fallback: if no recording event was observed before text arrives (e.g., component rendered mid-session, or the overlay is disabled), `elapsedSec` falls back to `10` seconds. This produces a conservative factor and never crashes.

## Win Formula
```
words   = value.trim().split(/\s+/).filter(Boolean).length
elapsed = (Date.now() - recordingStartedAt) / 1000   // seconds
seconds = Math.max(1, Math.round(elapsed))
factor  = Math.max(1, Math.round(words / Math.max(0.0167, elapsed) / (40/60)))
```
`40/60` converts 40 wpm to words-per-second. `Math.max(0.0167, elapsed)` (≈ 1 video frame) guards divide-by-zero for sub-frame elapsed times. `Math.max(1, ...)` clamps factor ≥ 1 so we never show "0× faster".

## Listener Cleanup
A single `listen<OverlayState>("show-overlay", handler)` is registered in a `useEffect`. The effect cleanup calls `unlisten.then((fn) => fn())`, matching the pattern used in `App.tsx`. The `hasComputedWin` ref prevents the win from being recomputed if the user edits the textarea after the initial paste.

## i18n Keys Added (`en/translation.json`)
```
onboarding.tryit.title      "Try it now"
onboarding.tryit.prompt     "Hold {{hotkey}} and read this line aloud."
onboarding.tryit.sample     "MindFlow turns my voice into text instantly."
onboarding.tryit.listening  "Listening…"
onboarding.tryit.win        "You typed {{words}} words in {{seconds}}s — about {{factor}}× faster than typing."
onboarding.tryit.continue   "Continue"
onboarding.tryit.skip       "Skip"
```

## Build / Lint Output
- `tsc --noEmit` → exit 0 (no type errors)
- `bun run build` → exit 0, 1940 modules transformed, built in 4m 45s
- `bun run lint` → exit 0 (no ESLint warnings)
- Pre-existing large-chunk warning (680 KB main bundle) unchanged

## Concerns
- **No integration in App.tsx yet** — Task 15 (per the plan) extends `OnboardingStep` to include `"tryit"` and wires this component into the flow. The component is intentionally self-contained and ready to drop in.
- **Textarea focus vs. hotkey focus** — On macOS, Tauri's webview window must be the frontmost window for the paste to land in the textarea. If the user switches apps to look something up, focus is lost. This is a UX risk to document in Task 15.
- **Push-to-talk only** — The component assumes push-to-talk semantics (show-overlay fires per recording). Toggle-mode users would not see the recording indicator on the first press (they release the key to stop). The textarea arrival detection still works either way.
