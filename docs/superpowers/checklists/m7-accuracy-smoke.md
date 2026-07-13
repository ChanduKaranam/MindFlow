# M7 accuracy-stack pre-release smoke (manual, real 0.6B model)

- [ ] Fresh install: onboarding offers cleanup model; skip path works; download path works
- [ ] Dictation with cleanup ON: "um so ship it friday no wait monday" → clean text, ≤ ~2s added latency (0.6B tier)
- [ ] Dictation with model deleted: rules-only fallback, no error surfaced
- [ ] Indian names: dictate 5 names from your dictionary on Parakeet AND Whisper — corrected on both
- [ ] Common-word guard: dictate "we import tea from china" with "Chandra" in dictionary → unchanged
- [ ] History: edit a mangled name → toast → dictionary entry → next dictation correct → undo removes entry
- [ ] Settings: all three flags off → LLM skipped (instant); sensitivity Off → no phonetic corrections
- [ ] Kill the app mid-download of the LLM → relaunch → partial download resumes/cleans up
- [ ] cargo test + CI green (mock swap unaffected)
- [ ] macOS: launch the bundled `.app` once and run a dictation with cleanup ON — the
      llama/ggml dylibs now live in `Contents/Frameworks` (dynamic-link fix; CI proves
      the build + bundle, but a real launch on hardware hasn't been done yet)
