import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

// DEV-ONLY: M1 injection harness — never imported in production builds.
// Calls deliver_text_cmd directly via raw invoke (not in generated bindings).
export default function DevInject() {
  const [text, setText] = useState("MindFlow injection test 123.");
  const [result, setResult] = useState("");
  return (
    <div style={{ padding: 16, border: "1px solid orange", borderRadius: 8 }}>
      <strong style={{ fontSize: 12, color: "orange" }}>
        DEV — M1 injection harness
      </strong>
      <div style={{ marginTop: 8 }}>
        <input
          value={text}
          onChange={(e) => setText(e.target.value)}
          style={{ width: "100%", marginBottom: 8 }}
        />
        <button
          onClick={async () => {
            setResult("Waiting 3s — focus a text field in another app…");
            setTimeout(async () => {
              try {
                const r = await invoke<string>("deliver_text_cmd", { text });
                setResult(`Result: ${r}`);
              } catch (e) {
                setResult(`Error: ${String(e)}`);
              }
            }, 3000);
          }}
        >
          Deliver in 3s
        </button>
      </div>
      {result && <p style={{ marginTop: 8, fontSize: 12 }}>{result}</p>}
    </div>
  );
}
