# M1 Injection + Hotkey Verification (run per OS)

For each of Windows, macOS, Linux-X11, Linux-Wayland:

## Injection
1. `cd app && bun run tauri dev`
2. In a dev build (`bun run tauri dev`), the DEV injection panel appears in the app window. Click "Deliver in 3s", then focus a text field in:
   - [ ] a browser address bar
   - [ ] a plain text editor (Notepad / TextEdit / gedit)
   - [ ] a chat app or terminal
3. Expect: the text appears in the focused field.
   - On Linux-Wayland: expect result = "clipboard" and the text is on the clipboard (paste manually with Ctrl+V).
   - Elsewhere: expect result = "pasted" and text inserted automatically.
4. [ ] Record OS, session type, result string, and pass/fail.

## Hotkey
5. With the app running, press the configured global shortcut while focused on another app.
   - [ ] The shortcut fires (recording overlay appears) — confirms Handy's global hotkey works in our fork.
6. [ ] If registration fails, the app surfaces an error (not a silent no-op).

## Gate
M1 passes when injection delivers text (pasted or clipboard) in all three app types
on all four session configurations, and the hotkey fires on each OS.

## Zero-network gate (carried forward)
`app/src-tauri/tests/zero_network.rs` MUST be extended in M2–M6 to drive the
full dictation flow with networking disabled and assert zero outbound
connections. Do not let later milestones ship without expanding it.
