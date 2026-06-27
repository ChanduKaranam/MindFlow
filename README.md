# MindFlow

**Free, fully-local, CPU-only voice dictation for Windows, macOS, and Linux.**
Hold a hotkey, speak, and your words appear — punctuated and formatted — in any
app. No GPU, no account, no cloud: your audio never leaves your device.

A Wispr Flow–style tool built on [Handy](https://github.com/cjpais/Handy) by cjpais.

---

## Download & install

Grab the installer for your OS from the [**Releases**](https://github.com/ChanduKaranam/MindFlow/releases) page:

| OS | File |
|----|------|
| Windows | `MindFlow_<version>_x64-setup.exe` (or `.msi`) |
| macOS (Apple Silicon) | `MindFlow_<version>_aarch64.dmg` |
| Linux | `MindFlow_<version>_amd64.AppImage` (or `.deb` / `.rpm`) |

> **These builds are currently unsigned.** On **Windows**, SmartScreen may say
> "Windows protected your PC" → click **More info → Run anyway**. On **macOS**,
> right-click the app → **Open** the first time to bypass Gatekeeper.

On first launch, MindFlow walks you through onboarding and downloads a speech
model sized for your CPU.

## Features

- **Dictate into any app** with a global hotkey (default `Ctrl/Cmd + Space`).
- **Local CPU speech-to-text** — Whisper / Parakeet / Moonshine, auto-tiered to
  your machine. Runs ≥ real-time on a typical laptop, no GPU.
- **Recording modes:** Hold (push-to-talk), Toggle, and **Hands-free** (tap to
  start, press **Enter** to stop & transcribe).
- **Formatting:** punctuation, capitalization, filler-word removal, spoken
  commands ("new line", "comma"…), number conversion.
- **Personalization:** custom-word dictionary, find/replace rules, and snippets.
- **Noise suppression** + voice-activity detection in the live pipeline.
- **100% local** — see *Privacy* below.

## Usage

1. Press the hotkey (or use your chosen Recording mode) and speak.
2. Release / press Enter; the transcribed, formatted text is typed into the
   focused app.
3. Tune everything in **Settings** — shortcut, recording mode, dictionary,
   snippets, model, noise suppression. Use the sidebar search to find a setting.

## Privacy

Core dictation makes **zero network calls**. The pipeline (capture → voice
detection → speech-to-text → formatting → text injection) runs entirely on your
device. The only network use is the **one-time model download** and an
**optional, disable-able** update check. Cloud post-processing is off by
default. This is enforced in CI by a guard test that fails the build if a
network call ever enters the dictation path — details in the
[zero-network audit](docs/superpowers/audits/2026-06-27-m6-zero-network-audit.md).

## Build from source

Prerequisites: [Rust](https://rustup.rs/) (stable) and [Bun](https://bun.sh/).

```bash
git clone https://github.com/ChanduKaranam/MindFlow.git
cd MindFlow/app
bun install

# One-time: fetch the bundled VAD + denoiser models
mkdir -p src-tauri/resources/models
curl -L -o src-tauri/resources/models/silero_vad_v6.onnx \
  https://github.com/snakers4/silero-vad/raw/v6.0/src/silero_vad/data/silero_vad.onnx
curl -L -o src-tauri/resources/models/gtcrn_simple.onnx \
  https://github.com/Xiaobin-Rong/gtcrn/raw/main/stream/onnx_models/gtcrn_simple.onnx

bun run tauri dev      # run in development
bun run tauri build    # produce installers in src-tauri/target/release/bundle/
```

See [`app/AGENTS.md`](app/AGENTS.md) for the full developer guide and
[`app/BUILD.md`](app/BUILD.md) for platform-specific build setup.

## Acknowledgments

MindFlow is built on [**Handy**](https://github.com/cjpais/Handy) by cjpais
(MIT). Speech-to-text is powered by whisper.cpp and the ONNX models credited in
the app's *About* screen.

## License

[MIT](LICENSE) — derivative of Handy (MIT). See [`LICENSE`](LICENSE).
