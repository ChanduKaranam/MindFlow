# MindFlow M5 — Rebrand, Visual Identity & First-Run UX (Design Spec)

**Date:** 2026-06-26
**Status:** Approved direction (pending spec review)
**Milestone:** M5 — "Make it MindFlow, make it feel effortless"

---

## 1. Purpose & Philosophy

MindFlow is a free, fully-local, CPU-only, privacy-first desktop voice-dictation app (a Wispr Flow clone) built on a vendored Handy fork. M1–M4 delivered the engine (hands-free mode, spoken-formatting commands, dictionary/replacements/snippets, noise suppression). The app still *looks and is named* "Handy," and none of what we built is discoverable.

M5 closes that gap on two axes the user named:

1. **Genuine value** — surface the quality/speed/privacy/features users actually want.
2. **Effortless UX** — a first-run and visual experience so familiar and polished that a new user feels they've used it for ages.

The design is grounded in a research pass (UX-psychology laws, desktop onboarding best practice, Wispr Flow's mental model, privacy-trust signaling, and voice-app visual identity). The five rules driving every decision below:

1. **Doherty Threshold (<400ms)** — feedback is instant; nothing feels like waiting. (Streaming transcription that fully honors this is deferred — see §9 — but all UI motion obeys it now.)
2. **Jakob's Law** — match Wispr's proven model: hold-to-talk + hands-free toggle, dictate-anywhere paste, tray icon + recording indicator, native-style preference layout.
3. **Peak-End Rule** — onboarding has one magical peak (your first words appear instantly in a real field) and a quantified end ("18 words in 6 seconds").
4. **Permission Priming (explain-before-prompt)** — never fire the one-shot OS prompt blind; show our own primer first.
5. **Trust-by-limitation + Tesler's Law** — "Your audio never leaves your device. No account. No cloud." is the hero message; the app absorbs setup complexity so the user just talks.

---

## 2. Global Constraints (bind every task)

- **CPU-only, fully local, zero network in the default path.** No new cloud calls. Fonts are **bundled locally as woff2** — never loaded from Google Fonts / any CDN (an offline app that reaches the network would break the core promise).
- **No new heavyweight dependencies.** Logo/icons are hand-authored SVG/PNG. No icon-CDN, no font-CDN, no animation library (use existing Tailwind transitions + CSS).
- **All user-facing strings via i18next** (ESLint `i18next/no-literal-string` enforces). New copy lands in `app/src/i18n/locales/en/translation.json` under namespaced keys; other locale files get the English fallback for new keys (translation of new strings is out of scope — fallback is acceptable and already the lookup behavior).
- **Bindings (`app/src/bindings.ts`) are hand-edited** — headless `tauri-specta` regen is unavailable. Any new Tauri command must be added to `collect_commands!` in `lib.rs` AND hand-mirrored into `bindings.ts`.
- **Settings plumbing pattern** (unchanged from M2–M4): field in `AppSettings` (`#[serde(default)]`) + entry in the `get_default_settings()` **struct literal** + a `change_*`/`update_*` command + register in `collect_commands!` + hand-edit `bindings.ts` + add a `settingsStore.ts` `settingUpdaters` entry (the generic fallback only `console.warn`s and does NOT persist).
- **Conventional commits** with trailer `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.
- **CI gate:** repo-root `.github/workflows/build.yml`, 3-OS matrix, `cargo check` only. Frontend lint/tsc are run locally per task.
- **Internal identifiers containing "handy" are NOT branding and MUST NOT be renamed:** `handy_keys`, `HandyKeysShortcutInput`, `startHandyKeysRecording`/`stopHandyKeysRecording`, the `handy_keys` keyboard-implementation value. These name a specific keyboard-injection backend; renaming breaks key handling.

---

## 3. Visual Identity — "Calm Capability, dark-first"

### 3.1 Color tokens

Replaces the current pink (`--color-background-ui: #da5893`) and pink-logo tokens in `app/src/App.css`. Warm-cool **gold + teal** on a dark-first canvas. Gold is the primary "capable" accent (brand, primary actions, active state); teal is the "live" color (recording/listening); a green confirms privacy; red is stop/cancel.

**Dark mode (default / `:root` baseline):**

| Token | Hex | Use |
|---|---|---|
| `--color-background` | `#121212` | App canvas (never pure black — avoids halation) |
| `--color-surface` | `#1C1C22` | Cards/panels (lighter = nearer; elevation by lightness, not shadow) |
| `--color-surface-high` | `#26262E` | Popovers, hover, higher elevation |
| `--color-text` | `#E6E6EA` | Primary text (never pure white) |
| `--color-text-secondary` | `#A1A1AA` | Captions / meta / descriptions |
| `--color-border` | `#33333B` | Hairlines, dividers, input borders |
| `--color-accent` (gold) | `#E0A53F` | Brand, primary buttons (with **dark** text), active nav |
| `--color-accent-hover` | `#EBB54E` | Hover on gold elements |
| `--color-accent-pressed` | `#C68A2E` | Pressed/gradient-deep |
| `--color-live` (teal) | `#2DD4BF` | Recording/listening indicator, "active model" |
| `--color-privacy` (green) | `#34D399` | "Local" / "audio never leaves" confirmations |
| `--color-danger` (red) | `#F87171` | Stop / cancel / destructive |
| `--color-on-accent` | `#121212` | Text/icons ON gold fills (gold+white fails WCAG; gold+dark ≈ 8.4:1) |

**Light mode (`@media (prefers-color-scheme: dark)` inverse — the existing media-query strategy is kept):**

| Token | Hex |
|---|---|
| `--color-background` | `#F7F7F8` |
| `--color-surface` | `#FFFFFF` |
| `--color-surface-high` | `#F0F0F2` |
| `--color-text` | `#18181B` |
| `--color-text-secondary` | `#52525B` |
| `--color-border` | `#E4E4E7` |
| `--color-accent` (gold) | `#E0A53F` (fills) / **`#9A6B12`** for gold *text/links* (gold-on-light text fails WCAG, so text uses dark amber; gold *fills* keep dark text) |
| `--color-live` (teal) | `#0D9488` (darkened for contrast on light) |
| `--color-privacy` | `#059669` |
| `--color-danger` | `#DC2626` |
| `--color-on-accent` | `#1A1305` |

**Compatibility:** `tailwind.config.js` currently maps `background`, `text`, `logo-primary`, `logo-stroke`, `text-stroke`. The new tokens are added to both `App.css` `@theme`/`:root` blocks and `tailwind.config.js`. The legacy `--color-background-ui` / `logo-*` tokens are **renamed/retired**; every component referencing them (e.g. `Button` `bg-background-ui`, `da5893`) is repointed to `--color-accent`. A grep sweep for `background-ui`, `logo-primary`, `logo-stroke`, `text-stroke`, and `da5893` enumerates the call sites.

The solid `--color-accent` (`#E0A53F`) above is the **fallback**; the primary brand expression is the **metallic gold gradient** in §3.4. Solid is used only where a gradient can't apply (16px tray glyph, single-color icons, focus rings, `currentColor` contexts).

**WCAG:** every text/background pairing in both themes must clear 4.5:1 (normal) / 3:1 (large + UI components). Gold fills always carry dark (`--color-on-accent`) text. Glass surfaces (§3.4) must be tested at worst-case (lightest possible background showing through) and stay ≥4.5:1 for the text they hold.

### 3.2 Typography

- **Geist** (UI workhorse — modern, precise, free OFL) for all body/controls.
- **Fraunces** (light optical serif, free OFL) for onboarding hero + section display headings only — adds editorial warmth, avoids the clinical-template look.
- **Geist Mono** for word-count, model names, and technical readouts.
- Bundled as **local woff2** under `app/src/assets/fonts/`, declared via `@font-face` in `App.css` with `font-display: swap`. No network fetch. `font-family` stacks fall back to the current system stack so a missing file never breaks layout.

### 3.3 Motion

All transitions **< 400ms** (Doherty). Use existing Tailwind transition utilities + CSS — no motion library.

- UI feedback (toggles, button press, nav active): **150ms**, standard easing `cubic-bezier(0.2,0,0,1)`.
- Panels / onboarding step transitions / overlays: **300ms**, emphasized-decelerate `cubic-bezier(0.05,0.7,0.1,1)`.
- The recording/live indicator pulses on the teal `--color-live`; a calm, slow pulse when idle-listening. (A full audio-reactive waveform is out of scope — §9 — a CSS pulse is the M5 deliverable.)
- **Gold sheen sweep** (decorative, not feedback): on hover of primary gold elements, a soft diagonal highlight band translates across once, ~600ms ease-out. This intentionally exceeds the 400ms feedback ceiling because it is *ambient decoration*, not interaction latency — the click/press state still fires <150ms. Honors `prefers-reduced-motion` (sheen + ambient drift disabled).

### 3.4 Material — reflective gold & glassmorphism

The premium feel comes from two materials layered on the dark canvas: **metallic gold** for brand/primary surfaces and **frosted glass** for chrome.

**3.4.1 Reflective ("metallic") gold.** Gradient tokens in `App.css`:

| Token | Value | Use |
|---|---|---|
| `--gradient-gold` | `linear-gradient(135deg, #A9760F 0%, #E0A53F 28%, #FBE7A1 50%, #E0A53F 72%, #A9760F 100%)` | Primary buttons, brand fills, logo. The `#FBE7A1` center stop is the **specular highlight** that reads as "shine." |
| `--gradient-gold-hover` | highlight shifted brighter/wider (`#FFF1C2` center, stops nudged) | Hover state |
| `--gold-edge-highlight` | `inset 0 1px 0 rgba(255,255,255,0.45)` | Top inner highlight on gold buttons (lit metal edge) |
| `--gold-edge-shadow` | `inset 0 -1px 0 rgba(0,0,0,0.30)` | Bottom inner shadow (depth) |

- **Logo / wordmark / gold text:** apply `--gradient-gold` via `background-clip: text; color: transparent`.
- **Gold buttons:** `background: var(--gradient-gold)` + both inset edge shadows + dark `--color-on-accent` text; hover swaps to `--gradient-gold-hover` and triggers the sheen sweep.
- **SVG logo (§4):** the gold node uses an SVG `linearGradient` (same three gold stops) plus a small white highlight ellipse for a true reflective look; the 16px tray glyph falls back to solid `--color-accent`.
- Contrast judged against the mid stop `#E0A53F` (dark text ≈ 8.4:1 ✓).

**3.4.2 Glassmorphism.** Frosted translucent chrome. Tokens in `App.css`:

| Token | Dark | Light | Notes |
|---|---|---|---|
| `--glass-bg` | `rgba(28,28,34,0.55)` | `rgba(255,255,255,0.60)` | Tinted enough to keep text contrast |
| `--glass-blur` | `blur(18px) saturate(150%)` | same | `backdrop-filter` |
| `--glass-border` | `1px solid rgba(255,255,255,0.10)` | `1px solid rgba(255,255,255,0.55)` | Glass edge; brighter top edge via inset highlight |
| `--glass-shadow` | `0 8px 32px rgba(0,0,0,0.35)` | `0 8px 32px rgba(0,0,0,0.12)` | Floating depth |

- **Applied to (chrome only):** settings **sidebar**, **cards / grouped panels** (`SettingsGroup` container — *not* each `SettingContainer` row), **modals** (incl. reset-confirm), **dropdowns / popovers / tooltips**, **onboarding step panels**, and the **recording overlay / pill**. Settings *rows* and dense lists stay on solid `--color-surface` for legibility and performance.
- **Ambient background layer (required for glass to read):** a new low-cost `AmbientBackground` element renders 2–3 large, very-low-opacity blurred radial glows (gold + teal) on `--color-background`, drifting slowly (CSS, `prefers-reduced-motion`-aware, paused when the window is hidden). Without it, blur over a flat color is invisible.
- **Fallback:** `@supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px)))` → glass surfaces fall back to opaque `--color-surface` + border + shadow (no blur). Always include `-webkit-backdrop-filter` for WKWebView / older WebKitGTK.
- **Performance guard (CPU-only):** cap simultaneous large blurred layers — one glass sidebar + one glass content container + transient modals/popovers. No glass nested in glass; no full-window blurred layer behind another blurred layer.

---

## 4. Logo & Icon Assets — "The Flow Mark"

### 4.1 The mark

A **mind node + waveform** in one composition: a rounded node (the "mind" — origin of thought) from which a single continuous waveform line flows rightward; the waveform's two center peaks subtly form an **M**. Voice (wave) + Mind (node) + Flow (unbroken line).

- **Shape language:** one filled node + one continuous open stroke, rounded caps, smooth Béziers (no sharp corners). Geometric, calm.
- **Color:** node in **reflective gold** — an SVG `linearGradient` using the §3.4.1 stops (`#A9760F → #E0A53F → #FBE7A1 → #E0A53F → #A9760F`) plus a small white highlight ellipse for a true metallic sheen; waveform stroke in **teal `#2DD4BF`** (two-tone warm/cool — avoids the muddy olive a blended gold→teal gradient would pass through). A monochrome variant (single `currentColor`) is provided for the 16px tray glyph and disabled states (no gradient at that size).
- **Buildable as clean SVG**, parameterized by `width`/`className`, mirroring the existing `HandyTextLogo` / `HandyHand` component API so swap-in is mechanical.

### 4.2 Deliverables

- **`FlowMark.tsx`** — the icon-only mark (replaces `HandyHand.tsx` usage as the sidebar/nav glyph).
- **`MindFlowLogo.tsx`** — mark + "MindFlow" wordmark (Geist/Fraunces letterforms or hand-tuned paths) (replaces `HandyTextLogo.tsx` in sidebar + onboarding).
- **App/bundle icons** — regenerate `app/src-tauri/icons/{32x32,64x64,128x128,128x128@2x}.png`, `icon.icns`, `icon.ico`, `logo.png`, and Windows Store square/wide assets from the mark. (PNG/ICO/ICNS generated from a master SVG via a documented step; committed as binaries.)
- **Tray/overlay assets** — regenerate `app/src-tauri/resources/{handy.png→mindflow.png, tray_idle{,_dark}.png, tray_recording{,_dark}.png, tray_transcribing{,_dark}.png, recording.png, transcribing.png}` from the mark in light/dark variants. Update `tauri.conf.json` resource paths and `tray.rs` icon references accordingly.

### 4.3 Preview-first gate

The first implementation task **renders a PNG preview of the mark** (and wordmark) and stops for user sign-off before any asset regeneration. The mark is approved as a rendered image, not a description.

---

## 5. Workstream A — Rebrand & Reskin

### 5.1 Identity / config (the "breaking" identity change the user approved)

| Location | From | To |
|---|---|---|
| `tauri.conf.json` `productName` | `Handy` | `MindFlow` |
| `tauri.conf.json` `identifier` | `com.pais.handy` | `com.mindflow.app` |
| `tauri.conf.json` updater endpoint | `github.com/cjpais/Handy/releases/.../latest.json` | MindFlow releases URL, or **updater disabled** if no release feed exists yet (decision §10) |
| `tauri.conf.json` signing display (`d Handy`) | `Handy` | `MindFlow` |
| `Cargo.toml` `description` | `Handy` | `MindFlow` |
| `package.json` `name` | `handy-app` | `mindflow-app` |
| Window `title` (`lib.rs`) | `Handy` | `MindFlow` |
| Tray tooltip (`tray.rs`) | `Handy v{version}` | `MindFlow v{version}` |

**Data-directory note:** changing `identifier` changes the OS app-data dir (Tauri derives it from the identifier), so existing models/settings on the dev machine are orphaned and re-downloaded — **explicitly accepted** by the user. No migration. (`Cargo.toml` package name `handy`/`handy_app_lib` is **internal**, not user-visible; it is left unchanged to avoid crate-rename churn and broken `handy_app_lib` references — decision §10.)

### 5.2 User-facing copy

Replace "Handy" → "MindFlow" in all user-visible strings:
- `app/src/i18n/locales/en/translation.json` (lines incl. permission descriptions, "Handy Shortcuts", autostart/tray descriptions, update-check, About version/data-dir/donate, acknowledgments, accessibility, language-setting). Other locale JSONs: the brand token "Handy" is a proper noun → replace the literal "Handy" occurrences in each locale file too (it's the same word in every language; safe mechanical replace), leaving surrounding translated text intact.
- `AboutSettings.tsx` GitHub URL → `github.com/ChanduKaranam/MindFlow`.
- `UpdateChecker.tsx` releases URL → MindFlow releases (or gated with the updater decision).
- Acknowledgment copy keeps credit to Whisper.cpp/Parakeet/Silero/Handy upstream (we are a fork — attribution stays; only the product name changes).

### 5.3 Reskin

- Swap color tokens (§3.1) in `App.css` + `tailwind.config.js`; repoint all `background-ui`/`logo-*`/`da5893` call sites to the new tokens.
- Add `@font-face` for Geist/Fraunces/Geist-Mono (§3.2).
- Replace logo components (§4.2); update imports in `Sidebar.tsx`, `Onboarding.tsx`, `AccessibilityOnboarding.tsx`.
- Verify both light/dark render and WCAG.

---

## 6. Workstream B — First-Run UX (the "peak")

Current onboarding (`App.tsx` + `onboarding/`) is two stages: permissions → model download. M5 reshapes it into a **≤5-step, stepper-guided flow** that primes permissions, delivers a peak, and surfaces features. Returning-user fast-path (permissions-only check) is preserved.

### 6.1 Steps

1. **Welcome / value-prop** (new) — Fraunces hero: *"Type at the speed of thought."* Subline + the trust triad rendered as three checked items: **"Your voice never leaves your device · No account · No cloud."** (privacy green checks). One primary gold "Get started" button (Von Restorff: the only filled button). Endowed-progress stepper shows "Step 1 of 5 — installed ✓".
2. **Permission primer — Microphone** (new primer wrapping the existing request) — our screen first: *"MindFlow needs your microphone so you can dictate into any app. Audio is processed on your device and never uploaded."* Then a button that fires the OS prompt (mic) / opens privacy settings (Windows). Existing polling/grant detection reused.
3. **Permission primer — Accessibility** (macOS only; new primer) — *"MindFlow needs Accessibility access so it can type transcribed text into other apps."* Guided, because macOS cannot grant this from an in-app dialog: numbered steps + a deep-link to System Settings → Privacy & Security → Accessibility, plus a "still off?" recovery state. (Windows/Linux skip this step; stepper adjusts total.)
4. **Model download** (existing, restyled) — keep the recommended-tier logic and progress UI; restyle to new tokens; add the stepper.
5. **Try it now + win** (new — the peak-end) — a focused text field with a pre-filled prompt: *"Hold {hotkey} and read this line aloud."* On successful transcription into the field, show the quantified win: *"You typed {N} words in {S}s — about {X}× faster than typing."* Then a **feature-intro** strip introducing what's included — **Hands-free mode, Spoken commands, Dictionary, Noise suppression** — each a one-line "what + how to find it." Final primary button: "Start using MindFlow."

### 6.2 Components & data

- **`OnboardingStepper.tsx`** (new) — presentational "Step X of N" with endowed progress; total adapts to platform (4 on Win/Linux, 5 on macOS).
- **`WelcomeStep.tsx`**, **`PermissionPrimer.tsx`** (parameterized: mic vs accessibility), **`TryItNowStep.tsx`**, **`FeatureIntro.tsx`** (new).
- `App.tsx` onboarding orchestration extended to sequence the new steps; existing permission-check / model-availability gating reused.
- **Try-it-now mechanics:** reuse the existing transcription pipeline — the step focuses its own text field, listens for the same delivered-text path used in production, counts words, and times from first-audio to delivered-text. If the user skips (a low-emphasis "Skip" link, Fitts-distant from primary), the win screen is bypassed and the feature-intro still shows. No new backend command if the existing delivered-text event can be observed by the frontend; otherwise a minimal read-only command exposes the last transcription word-count/duration (decision §10).
- **Persistence:** an `onboarding_completed: bool` setting (default `false`) gates the flow so it shows once. (Currently inferred from "any model available"; the explicit flag is more correct and lets us show the new steps even if a model is already present.) Standard settings plumbing (§2).
- **Material:** each onboarding step renders on a centered **glass panel** (§3.4.2) over the `AmbientBackground`; the single primary action per step is a **metallic-gold button** (§3.4.1, Von Restorff isolation).

### 6.3 Copy & i18n

All new strings under an extended `onboarding.*` namespace in `en/translation.json`. Permission copy uses the proven **"MindFlow needs [resource] so you can [benefit]"** formula.

---

## 7. Workstream C — Settings UX

Keep the existing 7-tab structure (General, Models, Advanced, History, Post-Processing, Debug, About). Add discoverability without restructuring. The settings **sidebar** and each **`SettingsGroup` card** render as **glass** (§3.4.2) over the `AmbientBackground`; individual rows stay solid for legibility/perf.

### 7.1 Settings search

- **`SettingsSearch.tsx`** (new) — a search field pinned at the top of the settings sidebar/content. Filters visible settings by title/description across tabs; matching settings surface with their tab label; non-matches hide. Pure client-side filter over a static index built from the settings' i18n titles/keywords. No backend.
- Implementation: a lightweight in-memory index mapping each setting (tab, group, i18n key, synonyms) → searchable text; the search filters and renders a flat results list with "in {tab}" context, clicking jumps to that tab with the row highlighted (Von Restorff highlight).

### 7.2 Reset to defaults

- **`change_*` is per-setting today; add a `reset_settings_to_defaults` command** (backend) that writes `get_default_settings()` and returns the fresh `AppSettings`; the frontend reloads the settings store. Standard plumbing + bindings.
- **`ResetDefaultsButton.tsx`** (new) — placed in About or Advanced, low-emphasis, **Fitts-distant** from common controls; opens a confirm modal (this is destructive). Reuses the existing confirm-modal pattern (model-delete already has one).

### 7.3 Group reorg (light)

- Rename/reorder groups so M1–M4 features are findable and logically chunked (Miller's Law, ~5 chunks per tab). Concretely: ensure **Hands-free / activation** controls read clearly in General; keep **Dictionary** (Custom Words, Replacements, Snippets) grouped in Advanced (already done in M4); ensure **Spoken commands** sits with activation/formatting; add a short **Privacy** affordance in About restating "100% local" (trust signal). No control is moved across tabs beyond what improves findability; no settings are removed.

---

## 8. Testing Strategy

- **Rust:** existing unit suites must stay green (`cargo test`); the `reset_settings_to_defaults` command gets a unit test asserting it returns struct-literal defaults. No audio-hardware paths added.
- **Frontend:** `tsc` + ESLint (incl. `i18next/no-literal-string`) clean on every task. New components are presentational and verified by build + manual.
- **Visual/manual (Windows + the user's machine):** logo preview sign-off; both light/dark themes; onboarding full run (fresh data dir) incl. permission primers, try-it-now peak, win screen, feature intro; settings search + reset.
- **Zero-network check:** confirm no font/icon/asset loads from a network origin (all local).
- **Glass/material check (per-OS, manual):** verify `backdrop-filter` renders on WebView2 (Windows), WKWebView (macOS), and WebKitGTK (Linux); confirm the `@supports` fallback yields opaque-but-correct chrome where unsupported; confirm `prefers-reduced-motion` disables sheen + ambient drift; spot-check that no scrolling jank appears with glass + ambient layer active (perf guard, §3.4.2).
- **CI:** 3-OS `cargo check` green before merge.

The visual/onboarding pieces cannot be unit-tested meaningfully — the gate is compile + suite + lint + manual verification, consistent with prior milestones.

---

## 9. Out of Scope (deferred, with rationale)

- **Streaming sub-400ms partial transcription** — the research's #1 "feel" rule, but a deep CPU-STT change, not polish. **Explicitly deferred to a post-M5 perf milestone**, per user ("after the UI is done").
- **Audio-reactive waveform** in the recording indicator — M5 ships a CSS pulse; the live-energy waveform is a later polish.
- Settings **import/export**, download **pause/resume**, **native menu bar**, **multi-window**, window-position persistence, translation-progress UI — YAGNI for v1.
- Translating the **new** onboarding/settings copy into the 15+ non-English locales — English fallback is acceptable; the existing brand-name replacement still applies to all locales.

---

## 10. Resolved Decisions

1. **Rebrand depth:** full identity (new `identifier` + data dir); models/settings re-downloaded on the dev machine; **no migration**. Internal Rust crate name (`handy`/`handy_app_lib`) left unchanged (invisible; rename = churn/risk).
2. **Logo:** Flow Mark — mind node (gold) + continuous waveform (teal) with a hidden M; preview-first sign-off gate (§4.3).
3. **Palette:** gold primary (`#E0A53F`) + teal live (`#2DD4BF`), dark-first; the rest of the research palette as in §3.1.
3a. **Material (user-directed):** the gold is expressed as a **reflective metallic gradient** (§3.4.1), and the UI uses **glassmorphism** on chrome surfaces (§3.4.2) over a required ambient background, with `@supports` fallback + a CPU-only performance guard.
4. **Onboarding:** includes the try-it-now peak + quantified win (user opted in).
5. **Updater URL:** if a MindFlow release feed does not yet exist, the updater is **pointed at the MindFlow repo releases path but tolerant of a missing feed** (the existing update-check already surfaces "no update"); it is not left pointing at `cjpais/Handy`. Final URL confirmed at implementation against the repo's release setup.
6. **Try-it-now data:** prefer observing the existing delivered-text path from the frontend; add a minimal read-only command only if the event isn't already frontend-visible. Resolved during planning against the M1 delivery code.

---

## 11. Decomposition note (for planning)

The three workstreams share the identity foundation (A) but are otherwise independent. Recommended execution order — **A (rebrand+reskin) → B (onboarding) → C (settings UX)** — because B and C render on top of A's tokens/logo. Planning may split this into one plan with three task groups or three sequential plans; either is acceptable. Workstream A's first task is the **logo-preview gate**.
