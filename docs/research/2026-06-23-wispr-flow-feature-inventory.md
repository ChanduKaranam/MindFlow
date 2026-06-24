# Wispr Flow — Complete Feature Inventory

*Research compiled 2026-06-23. Sources are official site/docs/app stores plus 2024–2026 third-party reviews. Items not confirmed from a primary source are flagged with ⚠.*

---

## 1. What it is

Wispr Flow is a system-wide AI voice-dictation product from Wispr AI (SF startup, founded 2021; macOS launch October 2024). You hold a hotkey, speak into any text field in any app, and Flow inserts cleaned-up, formatted text — not a verbatim transcript but an AI-edited rewrite (filler words removed, punctuation/capitalization added, tone adapted to the app). Markets itself as "4x faster than typing" (220 WPM vs ~45 WPM keyboard). **Critically for a local clone: all transcription and AI formatting happen in the cloud — it is not an on-device product.**

**Platforms:** macOS, Windows, iOS, Android. No Linux client. Cross-device sync of dictionary/snippets.

## 2. Core dictation features

- **Push-to-talk hotkey** — hold to dictate, release to insert; double-tap-toggle and hands-free modes too.
- **Configurable shortcuts** — up to 4 shortcuts, 3 keys each, modifier required.
- **Dictation anywhere** — works in any text field; markets 40+ named apps.
- **100+ languages**, auto-detected (no manual switching).
- **Whisper Mode** — works when you literally whisper.
- **Streaming pipeline** — text appears after release; ~500ms typical, vendor cites p99 < 700ms (cloud round-trip).
- **Accuracy** ~97.2% English; reviewers note AI cleanup sometimes over-edits/paraphrases.

## 3. AI / formatting features

- Auto-punctuation (from pauses/tone), auto-capitalization, paragraph formatting.
- Filler-word removal ("um/uh").
- Backtracking / self-correction ("meet at 2… actually 3").
- List formatting (spoken "1…2…3…" → numbered list).
- Tone/format adaptation per app (formal in docs, casual in Slack).
- **Command Mode (Pro, Mac/Win only)** — separate hotkey; rewrites selected text ("make concise", "translate"), answers questions inline, adjusts Flow's own settings by voice. Selection < 1000 words.
- **Context awareness** — reads active app/window (⚠ reportedly periodic screenshots of active window); recognizes camelCase/snake_case, CLI, filenames in Cursor/Windsurf.

## 4. Personalization

- Custom dictionary (auto-learns from corrections; manual jargon/names).
- Snippets / voice macros (cue → full pre-formatted text).
- Personal style/tone adaptation.
- Persistent style rules via Command Mode.
- Team-shared dictionary & snippets (Team/Enterprise).

## 5. Integrations & system behavior

- OS-level text insertion into focused field; no per-app integration.
- Clipboard fallback if direct insertion fails.
- Menu-bar (Mac) / system-tray (Windows) UI.
- iOS custom keyboard; Android input method.
- Onboarding: intro → language select → practice tap & hold-to-dictate → privacy prefs.
- Team dashboards, admin seats (Enterprise).

## 6. Account / cloud / pricing

| Tier | Price | Limits / features |
|---|---|---|
| Free (Basic) | $0 | 2,000 words/wk (Mac/Win), 1,000/wk (iPhone); dictionary + snippets; 100+ langs; Privacy Mode; HIPAA-ready |
| Pro | $15/mo ($12/mo annual) | Unlimited words; Command Mode; priority support; team collab; 14-day trial |
| Enterprise | Custom | SOC2/ISO27001; SSO/SAML; dashboards; HIPAA enforced |

**Everything runs in the cloud (ASR + LLM).** Subprocessors across reviews: Baseten, AWS, OpenAI, Anthropic, Cerebras, Meta/Llama. "Privacy Mode" = zero retention but cloud round-trip still happens.

## 7. Model details

- **Does NOT use OpenAI Whisper** as primary engine — proprietary undisclosed cloud ASR.
- **Formatting LLM = fine-tuned Meta Llama**, served on Baseten/AWS. 100+ tokens < 250ms; e2e p99 < 700ms.
- Exact ASR model + Llama version undocumented.

## 8. Feature priority for a free, local, CPU, OSS clone

| Feature | MVP priority | Local-CPU difficulty |
|---|---|---|
| Push-to-talk global hotkey | **Core** | Easy (OS global-hotkey APIs; cross-platform is the work) |
| OS-level text insertion | **Core** | Medium (per-OS accessibility/IME; clipboard-paste fallback cheap) |
| Clipboard fallback | **Core** | Easy |
| Streaming local ASR | **Core** | Medium-Hard (whisper.cpp/faster-whisper on CPU; small/base for latency; streaming is hard) |
| Auto-punctuation & capitalization | **Core** | Medium (Whisper emits punctuation) |
| Menu-bar / tray UI + settings | **Core** | Easy-Medium |
| Filler-word removal | **Important** | Easy-Medium (rules cheap; LLM cleaner) |
| Custom dictionary | **Important** | Medium (Whisper initial_prompt biasing / post replacement) |
| Snippets / voice macros | **Important** | Easy |
| List/markdown formatting | **Important** | Medium (rules or small LLM) |
| Multilingual (100+) | **Important** | Medium (Whisper multilingual; accuracy drops on small models) |
| Tone/format adaptation | **Nice-to-have** | Hard on CPU (needs local quantized 3–8B LLM) |
| AI rewriting (clean prose) | **Nice-to-have** | Hard on CPU (same local-LLM cost) |
| Command Mode | **Nice-to-have** | Hard (selection capture + local LLM) |
| Context awareness (active app) | **Nice-to-have** | Medium to detect app; avoid screenshots for privacy |
| Whisper-mode / backtracking / style learning | **Nice-to-have** | Hard |
| Team dictionary / sync | **Skip for MVP** | Conflicts with local-only goal |

**MVP recommendation:** local Whisper streaming ASR + global hotkey + text insertion/clipboard + auto-punctuation + dictionary biasing + snippets = ~80% of daily value, zero GPU, zero cloud. Tone adaptation, AI rewriting, and Command Mode are the differentiators needing a local quantized LLM (main CPU-performance risk).
