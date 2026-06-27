/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        // ── MindFlow monochrome-gold design tokens ──
        background: "var(--color-background)",
        surface: "var(--color-surface)",
        "surface-high": "var(--color-surface-high)",
        text: "var(--color-text)",
        "text-secondary": "var(--color-text-secondary)",
        border: "var(--color-border)",
        accent: "var(--color-accent)",
        "accent-hover": "var(--color-accent-hover)",
        "accent-pressed": "var(--color-accent-pressed)",
        recording: "var(--color-recording)",
        privacy: "var(--color-privacy)",
        danger: "var(--color-danger)",
        "on-accent": "var(--color-on-accent)",
      },
      fontFamily: {
        sans: [
          "Geist",
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "Segoe UI",
          "sans-serif",
        ],
        serif: [
          "Fraunces",
          "ui-serif",
          "Georgia",
          "serif",
        ],
        mono: [
          "Geist Mono",
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "monospace",
        ],
      },
    },
  },
  plugins: [],
};
