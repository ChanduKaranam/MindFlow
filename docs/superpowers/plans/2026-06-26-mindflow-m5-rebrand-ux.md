# MindFlow M5 — Rebrand, Visual Identity & First-Run UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the Handy fork into a branded, premium-feeling "MindFlow" with a reflective-gold + glassmorphism identity, a research-driven first-run experience, and discoverable settings.

**Architecture:** Three sequenced workstreams over a vendored Handy (Tauri 2 + Rust + React/TS + Tailwind v4) app. **A** swaps the design-token layer (`App.css` `@theme`), logo SVG components, app/tray icon assets, and all "Handy" identity/copy → MindFlow. **B** rebuilds the onboarding state machine in `App.tsx` into a stepper-guided, permission-primed flow with a "try it now" peak. **C** adds settings search + reset-to-defaults. Components are theme-agnostic, so the reskin is mechanical token repointing, not a rewrite.

**Tech Stack:** Tauri 2.x, Rust, React 18 + TypeScript (strict), Tailwind CSS v4 (`@theme` tokens), Zustand (`settingsStore`), i18next, lucide-react, hand-edited `bindings.ts`.

## Global Constraints

- **CPU-only, fully local, zero network in the default/runtime path.** Fonts are **bundled as local woff2** (`app/src/assets/fonts/`), never fetched from a CDN at runtime. (Fetching OFL fonts at dev time to *vendor* them into the repo is fine.)
- **No new heavyweight runtime dependencies.** Logo/icons are hand-authored SVG → committed PNG/ICO/ICNS. No animation library; use CSS + Tailwind transitions.
- **All user-facing strings via i18next** (ESLint `i18next/no-literal-string` enforces). New copy → `app/src/i18n/locales/en/translation.json` under namespaced keys; non-English locales get the English fallback for *new* keys.
- **Bindings (`app/src/bindings.ts`) are hand-edited.** Every new Tauri command: add to `collect_commands!` in `lib.rs:353` AND hand-mirror into `bindings.ts`.
- **Settings plumbing pattern:** field in `AppSettings` (`settings.rs:339`, `#[serde(default)]`) + entry in the `get_default_settings()` **struct literal** (`settings.rs:743`) + a `change_*`/`update_*` command (in `shortcut/mod.rs`, pattern of `change_noise_suppression_setting`) + register in `collect_commands!` + hand-edit `bindings.ts` + add a `settingsStore.ts` `settingUpdaters` entry (generic fallback only `console.warn`s — does NOT persist).
- **Do NOT rename internal identifiers containing "handy":** `handy_keys`, `HandyKeysShortcutInput`, `startHandyKeysRecording`/`stopHandyKeysRecording`, the `handy_keys` keyboard-impl value, and the Rust crate names `handy`/`handy_app_lib`. These are not branding; renaming breaks key handling / build.
- **Conventional commits**, trailer `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.
- **Per-task gate:** `cd app && bun run build` (tsc+vite) and `bun run lint` clean; `cd app/src-tauri && cargo test` green for backend tasks; `cargo check` is the 3-OS CI gate.
- **Theme tokens:** in Tailwind v4 the `@theme` block in `App.css` generates utilities (e.g. `--color-accent` → `bg-accent`). Add tokens there; `tailwind.config.js` `extend.colors` is mirrored for any legacy references.
- **WCAG:** text/bg ≥ 4.5:1 (normal) / 3:1 (large + UI). Gold fills carry dark `--color-on-accent` text. Glass tested at worst-case show-through.
- **Reduced motion:** `prefers-reduced-motion` disables the gold sheen sweep and ambient drift.

**Spec:** `docs/superpowers/specs/2026-06-26-mindflow-m5-rebrand-ux-design.md` (authoritative for exact token values).

---

## File Structure

**Workstream A — Rebrand & Reskin**
- `app/src/assets/brand/flowmark.svg` (NEW) — master logo SVG (source of truth for all raster assets).
- `app/src/assets/fonts/*.woff2` (NEW) — Geist, Geist Mono, Fraunces vendored.
- `app/src/App.css` (MODIFY) — replace `@theme` tokens; add `@font-face`, gold/glass utility classes, sheen + ambient keyframes, `@supports` fallback.
- `app/tailwind.config.js` (MODIFY) — mirror new color tokens.
- `app/src/components/ui/Button.tsx` (MODIFY) — metallic-gold primary + new tokens.
- `app/src/components/icons/FlowMark.tsx`, `MindFlowLogo.tsx` (NEW) — replace `HandyHand.tsx`/`HandyTextLogo.tsx`.
- `app/src/components/shared/AmbientBackground.tsx` (NEW).
- `app/src-tauri/icons/*`, `app/src-tauri/resources/*` (MODIFY) — regenerated from master SVG.
- `app/src-tauri/tauri.conf.json`, `Cargo.toml`, `app/package.json`, `app/src-tauri/src/lib.rs`, `tray.rs` (MODIFY) — identity strings.
- `app/src/i18n/locales/*/translation.json`, `AboutSettings.tsx`, `UpdateChecker.tsx` (MODIFY) — copy + URLs.

**Workstream B — Onboarding**
- `app/src-tauri/src/settings.rs`, `shortcut/mod.rs`, `lib.rs`, `bindings.ts`, `settingsStore.ts` (MODIFY) — `onboarding_completed`.
- `app/src/components/onboarding/OnboardingStepper.tsx`, `WelcomeStep.tsx`, `PermissionPrimer.tsx`, `TryItNowStep.tsx`, `FeatureIntro.tsx` (NEW).
- `app/src/App.tsx` (MODIFY) — onboarding state machine.

**Workstream C — Settings UX**
- `app/src-tauri/src/settings.rs`, `shortcut/mod.rs`, `lib.rs`, `bindings.ts`, `settingsStore.ts` (MODIFY) — `reset_settings_to_defaults`.
- `app/src/components/settings/ResetDefaultsButton.tsx`, `SettingsSearch.tsx` (NEW).
- `app/src/components/Sidebar.tsx`, `settings/about/AboutSettings.tsx` (MODIFY) — search mount, glass, privacy affordance.

---

# WORKSTREAM A — REBRAND & RESKIN

### Task 1: Logo master SVG + preview gate ⛔ CHECKPOINT

**Files:**
- Create: `app/src/assets/brand/flowmark.svg`, `app/src/assets/brand/mindflow-logo.svg` (mark + wordmark)
- Create (scratch, not committed): preview PNGs under the scratchpad

**Interfaces:**
- Produces: `flowmark.svg` (square, viewBox `0 0 48 48`, the mind-node+waveform mark) and `mindflow-logo.svg` (mark + "MindFlow" wordmark) — the visual source of truth Tasks 4 & 6 consume.

**Design (from spec §4):** A rounded **gold node** (the "mind") at the left from which a single continuous **teal waveform** flows right; the waveform's two center peaks subtly form an **M**. Node fill = SVG `linearGradient` gold stops `#A9760F → #E0A53F → #FBE7A1 → #E0A53F → #A9760F` + a small white highlight ellipse (reflective). Waveform = teal `#2DD4BF`, rounded caps, smooth Béziers. Provide a `currentColor` monochrome variant path for the 16px tray glyph.

- [ ] **Step 1: Author `flowmark.svg`** — hand-write clean SVG per the design. Include both the gradient gold node (with `<radialGradient>`/`<linearGradient>` + highlight ellipse) and the teal waveform path. Keep it legible at 16px (test by eye).
- [ ] **Step 2: Author `mindflow-logo.svg`** — the mark + "MindFlow" wordmark to its right (gold gradient on "Mind" via `fill="url(#gold)"`, `--color-text` on "Flow", or single-color — implementer's eye; must read premium).
- [ ] **Step 3: Render PNG previews** — using an available tool (`rsvg-convert`, ImageMagick `convert`, or `npx @resvg/resvg-js-cli`), render `flowmark.svg` at 256px and 16px and `mindflow-logo.svg` at 400px to the scratchpad. Also render a quick `sample.svg` mocking a **metallic gold button** and a **glass panel over an ambient glow** so the user approves the full material look.
- [ ] **Step 4: STOP — present previews to the user via the Read tool (image) and request sign-off.** Do not proceed to asset generation until approved. If changes requested, revise SVG and re-render.
- [ ] **Step 5: Commit the approved SVGs**

```bash
git add app/src/assets/brand/flowmark.svg app/src/assets/brand/mindflow-logo.svg
git commit -m "feat(brand): add MindFlow Flow Mark master SVGs (approved preview)"
```

---

### Task 2: Design tokens + fonts

**Files:**
- Modify: `app/src/App.css:3-12` (`@theme`), `:root`, dark `@media` block
- Modify: `app/tailwind.config.js:6-12`
- Create: `app/src/assets/fonts/{Geist-Regular,Geist-Medium,Geist-SemiBold,GeistMono-Regular,Fraunces-Light,Fraunces-Regular}.woff2`

**Interfaces:**
- Produces: CSS variables `--color-background|surface|surface-high|text|text-secondary|border|accent|accent-hover|accent-pressed|live|privacy|danger|on-accent`, `--gradient-gold|gradient-gold-hover|gold-edge-highlight|gold-edge-shadow`, `--glass-bg|glass-blur|glass-border|glass-shadow`; font families `Geist`, `Geist Mono`, `Fraunces`. Tasks 3–18 consume these.

- [ ] **Step 1: Vendor fonts** — download OFL woff2 for Geist + Geist Mono (Vercel, github.com/vercel/geist-font) and Fraunces (Google Fonts / github.com/undercasetype/Fraunces) at dev time; place the six files in `app/src/assets/fonts/`. (Vendored, not runtime-fetched.)
- [ ] **Step 2: Replace `@theme` tokens** in `App.css` — swap the pink block for the §3.1 dark-mode token set (this is the `:root`/default since dark is the design baseline values live in `@theme`+`:root`, light is the override — but the existing file uses light as `:root` and dark in the media query; KEEP that structure: put **light** values in `@theme`/`:root` and **dark** values in the `@media (prefers-color-scheme: dark)` block, using the spec's light table for `:root` and dark table for the media block). Add all new token names. Retain `--color-mid-gray` mapping (alias to `--color-text-secondary`) so legacy `mid-gray` utilities still resolve until repointed in Task 5.
- [ ] **Step 3: Add `@font-face`** for the three families (`font-display: swap`), and set `:root { font-family: "Geist", ui-sans-serif, system-ui, ...; }` with the existing system fallback retained.
- [ ] **Step 4: Mirror tokens in `tailwind.config.js`** `extend.colors` (`accent`, `surface`, `surface-high`, `text-secondary`, `border`, `live`, `privacy`, `danger`, `on-accent` → `var(--color-*)`), add `fontFamily.sans/serif/mono`. Keep legacy `logo-primary`/`logo-stroke`/`text-stroke` keys until Task 5 retires them.
- [ ] **Step 5: Verify** — `cd app && bun run build` succeeds; launch `bun run dev`, confirm the app background/text now use the new neutral tokens in both light/dark (toggle OS theme) with no console errors.
- [ ] **Step 6: Commit** — `feat(brand): new gold+teal dark-first design tokens and bundled fonts`

---

### Task 3: Reflective-gold + glass utilities + ambient background

**Files:**
- Modify: `app/src/App.css` (utility classes + keyframes)
- Create: `app/src/components/shared/AmbientBackground.tsx`

**Interfaces:**
- Consumes: tokens from Task 2.
- Produces: CSS classes `.glass` (frosted chrome), `.btn-gold` (metallic gold fill w/ edge insets), `.gold-text` (gradient clipped to text), `.sheen` (hover sweep); React `<AmbientBackground />` (fixed, behind content, `aria-hidden`). Tasks 4,5,11–18 consume.

- [ ] **Step 1: Add `.glass`** — `background: var(--glass-bg); backdrop-filter: var(--glass-blur); -webkit-backdrop-filter: var(--glass-blur); border: var(--glass-border); box-shadow: var(--glass-shadow);` with `@supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) { .glass { background: var(--color-surface); } }`.
- [ ] **Step 2: Add `.btn-gold`** — `background: var(--gradient-gold); color: var(--color-on-accent); box-shadow: var(--gold-edge-highlight), var(--gold-edge-shadow);` hover → `--gradient-gold-hover`. Add `.gold-text { background: var(--gradient-gold); -webkit-background-clip: text; background-clip: text; color: transparent; }`.
- [ ] **Step 3: Add sheen** — `.sheen` overlay (a `::after` diagonal `linear-gradient` highlight) animating `translateX` once on hover over ~600ms ease-out; wrap in `@media (prefers-reduced-motion: no-preference)`.
- [ ] **Step 4: Add ambient keyframes** — 2–3 slow-drifting blurred radial-gradient glows (gold + teal, low opacity) keyframes, guarded by `prefers-reduced-motion`.
- [ ] **Step 5: Build `AmbientBackground.tsx`** — a `position: fixed; inset: 0; z-index: -1; pointer-events: none;` `aria-hidden` element rendering the glow layers; pauses animation when `document.hidden` (visibilitychange listener).
- [ ] **Step 6: Verify** — temporarily mount `<AmbientBackground/>` + a `.glass` div + `.btn-gold` in `App.tsx`; `bun run dev`; confirm glass frosts over the glow and gold button shows the metallic sheen on hover (and that reduced-motion disables animation). Remove the temporary mount.
- [ ] **Step 7: Commit** — `feat(brand): metallic-gold + glass utilities and ambient background`

---

### Task 4: Logo React components

**Files:**
- Create: `app/src/components/icons/FlowMark.tsx`, `app/src/components/icons/MindFlowLogo.tsx`
- Modify: `app/src/components/Sidebar.tsx` (imports/usage at lines ~4-5, 37, 97), `app/src/components/onboarding/Onboarding.tsx` (~8,104), `app/src/components/onboarding/AccessibilityOnboarding.tsx` (~13,311)
- Delete: `app/src/components/icons/HandyHand.tsx`, `HandyTextLogo.tsx` (after references removed)

**Interfaces:**
- Consumes: `flowmark.svg` design (Task 1).
- Produces: `FlowMark({ width?: number; className?: string })` and `MindFlowLogo({ width?: number; className?: string })` — same call signature as the retired `HandyHand`/`HandyTextLogo` so swap-in is mechanical.

- [ ] **Step 1: Build `FlowMark.tsx`** — inline the approved `flowmark.svg` as a React component with `width`/`className` props (default `width=24`), gradient defs `id`-scoped to avoid collisions.
- [ ] **Step 2: Build `MindFlowLogo.tsx`** — inline `mindflow-logo.svg` similarly (used where `HandyTextLogo` was: sidebar header `width={120}`, onboarding `width={200}`).
- [ ] **Step 3: Repoint imports** — replace `HandyHand` → `FlowMark` and `HandyTextLogo` → `MindFlowLogo` in `Sidebar.tsx`, `Onboarding.tsx`, `AccessibilityOnboarding.tsx`. Grep `HandyTextLogo|HandyHand` to confirm zero remaining references, then delete the two old files.
- [ ] **Step 4: Verify** — `bun run build` + `bun run lint` clean; `bun run dev` shows the new mark in the sidebar and onboarding.
- [ ] **Step 5: Commit** — `feat(brand): replace Handy hand logo with MindFlow Flow Mark`

---

### Task 5: Reskin Button + retire legacy color tokens

**Files:**
- Modify: `app/src/components/ui/Button.tsx:24-37`
- Modify: every call site of `bg-background-ui`/`logo-primary`/`logo-stroke`/`text-stroke`/`mid-gray`/`da5893` (enumerate via grep)
- Modify: `app/src/App.css` + `tailwind.config.js` — remove the legacy `--color-background-ui`/`logo-*`/`text-stroke` tokens and the `mid-gray` alias once unreferenced

**Interfaces:**
- Consumes: tokens (Task 2), `.btn-gold`/`.gold-text` (Task 3).
- Produces: `Button` `primary` variant rendered as metallic gold; other variants on new tokens.

- [ ] **Step 1: Grep the surface area** — `cd app && grep -rn "background-ui\|logo-primary\|logo-stroke\|text-stroke\|mid-gray\|da5893" src` → produces the authoritative repoint list.
- [ ] **Step 2: Rewrite `Button.tsx` variants** — `primary` → `.btn-gold` (drop `text-white bg-background-ui …`, use `text-on-accent` + the gold classes; keep `.sheen` on hover); `primary-soft` → `bg-accent/15 text-text hover:bg-accent/25 focus:ring-accent`; `secondary` → `bg-surface-high border-border hover:bg-surface-high/70`; `danger` keep but map to `--color-danger`; `danger-ghost`/`ghost` → repoint `mid-gray`→`text-secondary`, `logo-primary`→`accent`.
- [ ] **Step 3: Repoint remaining call sites** from Step 1 (e.g. `App.tsx` Toaster `border-mid-gray/20`→`border-border`, `text-mid-gray`→`text-text-secondary`).
- [ ] **Step 4: Remove dead tokens** from `App.css`/`tailwind.config.js` after grep confirms zero references to the legacy names.
- [ ] **Step 5: Verify** — `bun run build` + `bun run lint` clean; `bun run dev`: primary buttons are metallic gold with dark text + sheen; no pink remains anywhere; both themes pass a visual contrast check.
- [ ] **Step 6: Commit** — `feat(brand): reskin Button to metallic gold and retire pink tokens`

---

### Task 6: Regenerate app + tray icon assets

**Files:**
- Modify (regenerate): `app/src-tauri/icons/{32x32,64x64,128x128,128x128@2x}.png`, `icon.icns`, `icon.ico`, `logo.png`, Windows Store `Square*Logo.png`/`StoreLogo.png`
- Modify (regenerate + rename): `app/src-tauri/resources/{handy.png→mindflow.png, tray_idle{,_dark},tray_recording{,_dark},tray_transcribing{,_dark},recording,transcribing}.png`
- Modify: `app/src-tauri/tauri.conf.json:30-38` (resource/icon paths), `app/src-tauri/src/tray.rs` (icon filename refs), any `lib.rs`/`overlay.rs` reference to `handy.png`

**Interfaces:**
- Consumes: master SVGs (Task 1).
- Produces: branded raster assets at all required sizes; updated paths in config/code.

- [ ] **Step 1: Generate the app icon set** — render `mindflow-logo`/`flowmark` to a 1024px master PNG, then `cd app && bun run tauri icon path/to/master-1024.png` to regenerate `src-tauri/icons/*` (handles ico/icns/png/store sizes). Confirm the generated files overwrite the old Handy icons.
- [ ] **Step 2: Generate tray/overlay resources** — render `flowmark.svg` (and its state-tinted variants: idle = gold/teal, recording = teal `--color-live`, transcribing = gold) to PNG at the tray sizes for both light and dark, writing the `resources/*` filenames. Rename `handy.png`→`mindflow.png`.
- [ ] **Step 3: Update references** — grep `app/src-tauri` for `handy.png` and the tray resource names; repoint `tauri.conf.json` and `tray.rs`/`overlay.rs` to the new filenames. Update the `icon` array in `tauri.conf.json` only if filenames changed (they don't — `tauri icon` keeps names).
- [ ] **Step 4: Verify** — `cd app/src-tauri && cargo check` passes (resource paths resolve); `bun run dev`: the tray shows the new mark and switches state icons (idle/recording/transcribing).
- [ ] **Step 5: Commit** — `feat(brand): regenerate app and tray icons from Flow Mark`

---

### Task 7: Identity / config rebrand (the breaking change)

**Files:**
- Modify: `app/src-tauri/tauri.conf.json` (`productName:3`, `identifier:5`, signing `d Handy:61`, updater endpoint `:71`)
- Modify: `app/src-tauri/Cargo.toml:4` (`description`)
- Modify: `app/package.json:2` (`name`)
- Modify: `app/src-tauri/src/lib.rs:559` (window `.title`)
- Modify: `app/src-tauri/src/tray.rs:90,92` (tooltip strings)

**Interfaces:**
- Produces: app identity = MindFlow; bundle id `com.mindflow.app` (changes the OS data dir — accepted, no migration).

- [ ] **Step 1:** `tauri.conf.json` — `productName` → `"MindFlow"`; `identifier` → `"com.mindflow.app"`; signing display `-d Handy` → `-d MindFlow`; updater endpoint → `"https://github.com/ChanduKaranam/MindFlow/releases/latest/download/latest.json"` (the existing update-check tolerates a missing/empty feed; this points off `cjpais/Handy`). **Leave the `pubkey` unchanged** (re-signing is out of scope; note in commit body that a real release requires a new keypair).
- [ ] **Step 2:** `Cargo.toml` `description = "MindFlow"`. Leave `name = "handy"`/`handy_app_lib` (internal, per constraints).
- [ ] **Step 3:** `package.json` `"name": "mindflow-app"`.
- [ ] **Step 4:** `lib.rs:559` `.title("MindFlow")`.
- [ ] **Step 5:** `tray.rs` tooltip → `"MindFlow v{} (Dev)"` / `"MindFlow v{}"`.
- [ ] **Step 6: Verify** — `cd app/src-tauri && cargo check` passes; `cd app && bun run build` passes. (Data-dir reset means a fresh first run — expected.)
- [ ] **Step 7: Commit** — `feat(brand)!: rebrand identity to MindFlow (new bundle id, data dir resets)`

---

### Task 8: User-facing copy + URL rebrand

**Files:**
- Modify: `app/src/i18n/locales/*/translation.json` (every locale; literal "Handy" → "MindFlow")
- Modify: `app/src/components/settings/about/AboutSettings.tsx:69` (GitHub URL)
- Modify: `app/src/components/update-checker/UpdateChecker.tsx:206` (releases URL)

**Interfaces:**
- Produces: zero user-visible "Handy" strings remain.

- [ ] **Step 1: Replace brand token in all locales** — for each `app/src/i18n/locales/*/translation.json`, replace the literal proper noun `Handy` with `MindFlow` (it's the same word in every language; surrounding translated text is untouched). Use a scripted replace, then `git diff` to confirm only "Handy"→"MindFlow" changed.
- [ ] **Step 2: Acknowledgments** — verify the About acknowledgments still credit Whisper.cpp/Parakeet/Silero and upstream Handy *as attribution* (we are a fork). If the only "Handy" there is the product self-reference, it becomes MindFlow; keep a one-line "based on Handy by cjpais" attribution if present.
- [ ] **Step 2b:** `AboutSettings.tsx` GitHub URL → `https://github.com/ChanduKaranam/MindFlow`.
- [ ] **Step 3:** `UpdateChecker.tsx` releases URL → `https://github.com/ChanduKaranam/MindFlow/releases/latest`.
- [ ] **Step 4: Verify** — `cd app && grep -rn "Handy" src/i18n src/components | grep -vi "handy_keys\|HandyKeys\|cjpais"` returns only intentional attribution; `bun run build` + `bun run lint` clean.
- [ ] **Step 5: Commit** — `feat(brand): rebrand all user-facing copy and links to MindFlow`

---

# WORKSTREAM B — FIRST-RUN UX

### Task 9: `onboarding_completed` setting

**Files:**
- Modify: `app/src-tauri/src/settings.rs` (struct ~339; defaults ~743), `app/src-tauri/src/shortcut/mod.rs` (new command), `app/src-tauri/src/lib.rs:353` (`collect_commands!`)
- Modify: `app/src/bindings.ts`, `app/src/stores/settingsStore.ts`

**Interfaces:**
- Produces: `AppSettings.onboarding_completed: bool` (default `false`); command `set_onboarding_completed(completed: bool) -> Result<(), String>`; binding `commands.setOnboardingCompleted`; store updater key `onboarding_completed`.

- [ ] **Step 1: Add field** — in `AppSettings`: `#[serde(default)] pub onboarding_completed: bool,`; in `get_default_settings()` struct literal: `onboarding_completed: false,`.
- [ ] **Step 2: Unit test** — in `settings.rs` `#[cfg(test)]`: `assert!(!get_default_settings().onboarding_completed);` Run `cargo test onboarding -- --nocapture` → PASS.
- [ ] **Step 3: Add command** in `shortcut/mod.rs` mirroring `change_noise_suppression_setting`:
```rust
#[tauri::command]
#[specta::specta]
pub fn set_onboarding_completed(app: AppHandle, completed: bool) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app);
    settings.onboarding_completed = completed;
    crate::settings::write_settings(&app, settings);
    Ok(())
}
```
- [ ] **Step 4: Register** — add `shortcut::set_onboarding_completed,` to `collect_commands![` in `lib.rs:353`.
- [ ] **Step 5: Bindings + store** — hand-add `async setOnboardingCompleted(completed: boolean)` to `bindings.ts` (mirror an existing bool command exactly), add `onboarding_completed?: boolean` to the `AppSettings` type, and add `onboarding_completed` to `settingUpdaters` in `settingsStore.ts` calling `commands.setOnboardingCompleted`.
- [ ] **Step 6: Verify** — `cargo test` green, `cd app && bun run build` clean.
- [ ] **Step 7: Commit** — `feat(onboarding): add onboarding_completed setting + command`

---

### Task 10: OnboardingStepper component

**Files:** Create `app/src/components/onboarding/OnboardingStepper.tsx`

**Interfaces:**
- Produces: `OnboardingStepper({ current: number; total: number; labels?: string[] })` — presentational "Step X of N" with endowed-progress fill (first segment pre-filled). Consumed by Tasks 11–13, 15.

- [ ] **Step 1: Build** — a horizontal segmented progress bar; completed/current segments fill with `--color-accent` (gold), upcoming with `--color-border`; an accessible `aria-label` "Step {current} of {total}" and `aria-valuenow`. All text via i18next (`onboarding.stepper.label` with `{{current}}`/`{{total}}`).
- [ ] **Step 2: Add i18n key** — `onboarding.stepper.label` to `en/translation.json`.
- [ ] **Step 3: Verify** — `bun run build` + `bun run lint` clean.
- [ ] **Step 4: Commit** — `feat(onboarding): add OnboardingStepper`

---

### Task 11: WelcomeStep (value-prop + trust triad)

**Files:** Create `app/src/components/onboarding/WelcomeStep.tsx`; add `onboarding.welcome.*` to `en/translation.json`

**Interfaces:**
- Consumes: `.glass`, `AmbientBackground`, `.btn-gold`, `MindFlowLogo`, `OnboardingStepper`.
- Produces: `WelcomeStep({ onContinue: () => void; stepIndex: number; stepTotal: number })`.

- [ ] **Step 1: Build** — full-screen layout with `<AmbientBackground/>` behind a centered `.glass` panel: `MindFlowLogo`, a Fraunces hero (`onboarding.welcome.title` = "Type at the speed of thought."), subline, and a **trust triad** rendered as three rows with a privacy-green check icon (lucide `Check`): "Your voice never leaves your device", "No account", "No cloud" (`onboarding.welcome.trust.{local,noAccount,noCloud}`). One primary `.btn-gold` "Get started" (the only filled button — Von Restorff). `OnboardingStepper current={stepIndex} total={stepTotal}` at top.
- [ ] **Step 2: i18n** — add all `onboarding.welcome.*` keys.
- [ ] **Step 3: Verify** — `bun run build`+`lint` clean; `bun run dev` (temporarily render it) shows the glass/gold/ambient look in both themes.
- [ ] **Step 4: Commit** — `feat(onboarding): add WelcomeStep value-prop screen`

---

### Task 12: PermissionPrimer (explain-before-prompt)

**Files:** Create `app/src/components/onboarding/PermissionPrimer.tsx`; add `onboarding.primer.*` keys

**Interfaces:**
- Consumes: existing permission APIs already used in `AccessibilityOnboarding.tsx` (`requestMicrophonePermission`, `requestAccessibilityPermission`, `openMicrophonePrivacySettings`, the macOS-permissions-api checks) and `getWindowsMicrophonePermissionStatus`.
- Produces: `PermissionPrimer({ kind: "microphone" | "accessibility"; onGranted: () => void; onSkip?: () => void; stepIndex; stepTotal })`.

- [ ] **Step 1: Build** — a `.glass` panel that, *before* any OS prompt, explains with the formula **"MindFlow needs {resource} so you can {benefit}"** (`onboarding.primer.microphone.*`, `onboarding.primer.accessibility.*`). A primary `.btn-gold` "Allow {resource}" that THEN fires the real request (mic: `requestMicrophonePermission()` / Windows: `openMicrophonePrivacySettings()`; accessibility/macOS: deep-link to System Settings + numbered steps, since it can't be granted in-app). Reuse the existing polling pattern from `AccessibilityOnboarding.tsx` to detect grant and call `onGranted`. Include a "still off?" recovery hint for macOS accessibility. A low-emphasis `onSkip` link, visually distant from the primary (Fitts).
- [ ] **Step 2: i18n** — add `onboarding.primer.*` (microphone + accessibility copy, "still off" recovery, allow/skip labels).
- [ ] **Step 3: Verify** — `bun run build`+`lint` clean.
- [ ] **Step 4: Commit** — `feat(onboarding): add explain-before-prompt PermissionPrimer`

---

### Task 13: TryItNowStep + quantified win (peak-end)

**Files:** Create `app/src/components/onboarding/TryItNowStep.tsx`; add `onboarding.tryit.*` keys

**Interfaces:**
- Consumes: the production transcription delivery path. **Resolve at implementation:** check `app/src/components/DevInject.tsx` and the M1 delivery code for how transcribed text reaches a focused field / which event fires on delivery (`listen("...")`). If a frontend-observable event with text exists, use it; otherwise capture into the step's own `<textarea>` and read its value/word-count directly.
- Produces: `TryItNowStep({ hotkey: string; onDone: () => void; stepIndex; stepTotal })`.

- [ ] **Step 1: Build the practice field** — a `.glass` panel with a focused `<textarea>` and a pre-filled prompt line: `onboarding.tryit.prompt` = "Hold {{hotkey}} and read this line aloud." Show the live recording indicator (teal pulse) while recording.
- [ ] **Step 2: Detect success + compute the win** — when transcribed text lands in the field (non-empty), compute `words = text.trim().split(/\s+/).length` and elapsed seconds from first-audio to delivery; show the win: `onboarding.tryit.win` = "You typed {{words}} words in {{seconds}}s — about {{factor}}× faster than typing." (`factor` = round(words / (seconds * 40/60)) clamped ≥ 1, i.e. vs ~40 wpm typing; guard divide-by-zero).
- [ ] **Step 3: Skip path** — a Fitts-distant "Skip" link → `onDone()` without the win (feature-intro still shows next).
- [ ] **Step 4: i18n** — add `onboarding.tryit.*`.
- [ ] **Step 5: Verify** — `bun run build`+`lint` clean. (Live dictation verified manually on the user's machine.)
- [ ] **Step 6: Commit** — `feat(onboarding): add try-it-now practice + quantified win`

---

### Task 14: FeatureIntro strip

**Files:** Create `app/src/components/onboarding/FeatureIntro.tsx`; add `onboarding.features.*` keys

**Interfaces:**
- Produces: `FeatureIntro({ onFinish: () => void; stepIndex; stepTotal })`.

- [ ] **Step 1: Build** — a `.glass` panel listing the four M1–M4 features as one-line cards with a lucide icon each: **Hands-free mode**, **Spoken commands**, **Dictionary**, **Noise suppression** — each "what it does + where to find it" (`onboarding.features.{handsFree,spokenCommands,dictionary,noiseSuppression}.{title,desc}`). Final primary `.btn-gold` "Start using MindFlow" → `onFinish()`.
- [ ] **Step 2: i18n** — add `onboarding.features.*`.
- [ ] **Step 3: Verify** — `bun run build`+`lint` clean.
- [ ] **Step 4: Commit** — `feat(onboarding): add feature-intro strip`

---

### Task 15: Wire onboarding state machine

**Files:** Modify `app/src/App.tsx:26,36,173-253` and `app/src/components/onboarding/index.ts`; restyle `Onboarding.tsx` (model step) to new tokens

**Interfaces:**
- Consumes: Tasks 9–14 components + `commands.setOnboardingCompleted`.

- [ ] **Step 1: Extend the step type** — `type OnboardingStep = "welcome" | "accessibility" | "model" | "tryit" | "done"`. Compute `stepTotal` per platform (macOS = 5: welcome, mic primer, accessibility primer, model, tryit+features; Win/Linux = 4: welcome, mic primer, model, tryit+features).
- [ ] **Step 2: Gate on `onboarding_completed`** — in `checkOnboardingStatus`, a NEW user (no models AND `!settings.onboarding_completed`) starts at `"welcome"`. Returning users (`onboarding_completed === true`) keep the existing permissions-only fast-path → `"done"`. Preserve all existing macOS/Windows permission-check branches.
- [ ] **Step 3: Sequence render** — render `WelcomeStep` → `PermissionPrimer kind="microphone"` → (macOS) `PermissionPrimer kind="accessibility"` → `Onboarding` (model) → `TryItNowStep` then `FeatureIntro` → on finish call `commands.setOnboardingCompleted(true)` and set `"done"`. Replace the old `AccessibilityOnboarding` direct render with the primer sequence (keep `AccessibilityOnboarding` available for the returning-user permission-repair path, or route that through `PermissionPrimer` too — implementer's call, but returning users must still be able to fix permissions).
- [ ] **Step 4: Restyle model step** — repoint `Onboarding.tsx` (the model-download screen) to new tokens + `.glass` panel + `OnboardingStepper`; keep its recommended-tier logic intact.
- [ ] **Step 5: Verify** — `bun run build`+`lint` clean; `bun run dev` with a fresh data dir walks welcome → primers → model → try-it → features → app. Returning-user path (set `onboarding_completed`, keep a model) skips to app.
- [ ] **Step 6: Commit** — `feat(onboarding): wire stepper-guided primed first-run flow`

---

# WORKSTREAM C — SETTINGS UX

### Task 16: Reset-to-defaults command + UI

**Files:**
- Modify: `app/src-tauri/src/shortcut/mod.rs` (command), `lib.rs:353`, `bindings.ts`, `settingsStore.ts`
- Create: `app/src/components/settings/ResetDefaultsButton.tsx`; mount in `AboutSettings.tsx`
- Add `settings.reset.*` keys

**Interfaces:**
- Produces: `reset_settings_to_defaults() -> Result<AppSettings, String>` (writes `get_default_settings()`, returns it); binding `commands.resetSettingsToDefaults`; a confirm-modal-guarded button.

- [ ] **Step 1: Command + test** — add:
```rust
#[tauri::command]
#[specta::specta]
pub fn reset_settings_to_defaults(app: AppHandle) -> Result<crate::settings::AppSettings, String> {
    let defaults = crate::settings::get_default_settings();
    crate::settings::write_settings(&app, defaults.clone());
    Ok(defaults)
}
```
Unit test in `settings.rs`/`shortcut` asserting `get_default_settings()` round-trips (e.g. equals a fresh call). `cargo test` PASS.
- [ ] **Step 2: Register** in `collect_commands!`.
- [ ] **Step 3: Bindings + store** — hand-add `async resetSettingsToDefaults()` returning `Result<AppSettings, ...>` to `bindings.ts`; after success the store reloads settings (call the existing settings-load path so all controls reflect defaults).
- [ ] **Step 4: Build `ResetDefaultsButton.tsx`** — a low-emphasis `danger-ghost` button, placed at the **bottom of About**, Fitts-distant from common controls; opens the existing confirm-modal pattern (as used by model-delete) with `settings.reset.confirm.*` copy; on confirm calls the command and reloads.
- [ ] **Step 5: i18n** — add `settings.reset.*`.
- [ ] **Step 6: Verify** — `cargo test` green; `bun run build`+`lint` clean; `bun run dev`: changing a setting then Reset restores defaults after confirm.
- [ ] **Step 7: Commit** — `feat(settings): add reset-to-defaults command and guarded button`

---

### Task 17: Settings search

**Files:** Create `app/src/components/settings/SettingsSearch.tsx`; mount in `app/src/components/Sidebar.tsx`; add `settings.search.*` keys

**Interfaces:**
- Consumes: `SECTIONS_CONFIG` / the sidebar section list (`Sidebar.tsx`), `i18n` titles.
- Produces: `SettingsSearch({ onJump: (section: SidebarSection) => void })`.

- [ ] **Step 1: Build a static index** — a module mapping each searchable setting → `{ section, titleKey, keywords[] }` (covering the headline settings + the M1–M4 features: hands-free, spoken commands, dictionary, noise suppression). Translate titles via `i18n.t` at search time.
- [ ] **Step 2: Build the search field** — pinned at the top of the sidebar; as the user types, show a dropdown of matches (case-insensitive substring over translated title + keywords) labelled "{title} — in {sectionLabel}"; selecting one calls `onJump(section)` (which sets `currentSection`), and the target row highlights briefly (`--color-accent` ring, Von Restorff). Empty query → no dropdown.
- [ ] **Step 3: Wire** — `Sidebar.tsx` passes `onJump={onSectionChange}`; mount above the section nav.
- [ ] **Step 4: i18n** — `settings.search.placeholder`, `settings.search.inSection` (`{{section}}`), `settings.search.noResults`.
- [ ] **Step 5: Verify** — `bun run build`+`lint` clean; `bun run dev`: typing "dictionary" surfaces it and jumps to Advanced; "hands" surfaces hands-free.
- [ ] **Step 6: Commit** — `feat(settings): add cross-tab settings search`

---

### Task 18: Glass chrome + group reorg + privacy affordance

**Files:** Modify `app/src/components/Sidebar.tsx`, `app/src/components/ui/SettingsGroup.tsx`, `app/src/App.tsx` (mount `AmbientBackground`), `app/src/components/settings/about/AboutSettings.tsx`, and the general/advanced group ordering where it improves findability

**Interfaces:**
- Consumes: `.glass`, `AmbientBackground` (Task 3).

- [ ] **Step 1: Mount ambient** — render `<AmbientBackground/>` once at the root of the main app view in `App.tsx` (behind sidebar + content).
- [ ] **Step 2: Glass the chrome** — apply `.glass` to the `Sidebar` container and the `SettingsGroup` card container (NOT individual rows). Verify text contrast holds over the ambient glow (worst case).
- [ ] **Step 3: Light reorg** — ensure activation/formatting settings (hands-free recording-mode, spoken commands) read clearly in General; Dictionary stays grouped in Advanced (already from M4); no setting removed; ≤5 groups per tab where feasible (Miller).
- [ ] **Step 4: Privacy affordance** — add a small "100% local — your audio never leaves your device" line with a privacy-green check at the top of About (`settings.about.privacy`), a quiet trust signal.
- [ ] **Step 5: i18n** — add `settings.about.privacy`.
- [ ] **Step 6: Verify** — `bun run build`+`lint` clean; `bun run dev`: sidebar + cards are frosted glass over the drifting glow in both themes, scrolling stays smooth (perf guard), `@supports` fallback verified by toggling (or on a webview without backdrop-filter).
- [ ] **Step 7: Commit** — `feat(ui): glass chrome, ambient background, group reorg, privacy line`

---

## Self-Review

**Spec coverage:** §3.1 tokens → T2; §3.2 fonts → T2; §3.3 motion/sheen → T3; §3.4.1 metallic gold → T3/T5; §3.4.2 glass + ambient + fallback + perf → T3/T18; §4 logo + preview gate → T1/T4; §4.2 icon assets → T6; §5.1 identity → T7; §5.2 copy → T8; §5.3 reskin → T2/T5; §6.1 steps → T11–T15; §6.2 components/persistence → T9–T15; §6.3 copy → per-task i18n; §7.1 search → T17; §7.2 reset → T16; §7.3 reorg/privacy → T18; §8 testing → per-task gates + manual; §9 out-of-scope respected (no streaming transcription, no audio-reactive waveform). **No gaps.**

**Placeholders:** Two items are intentionally resolved at implementation against existing code (updater feed URL exact form — T7; try-it-now delivery-event source — T13). Both name the exact file to inspect and the decision rule, per spec §10. Not vague placeholders.

**Type consistency:** `onboarding_completed` / `setOnboardingCompleted` consistent T9↔T15; `reset_settings_to_defaults` / `resetSettingsToDefaults` consistent T16; `FlowMark`/`MindFlowLogo` signatures consistent T1/T4↔consumers; `.glass`/`.btn-gold`/`.gold-text`/`AmbientBackground` defined T3, consumed by name in T5,11–14,18; `OnboardingStepper` props consistent T10↔T11–13,15.

**Note:** Task 1 is a human-approval CHECKPOINT (preview sign-off) — execution pauses there for the user before proceeding, by design.
