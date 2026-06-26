import { useEffect, useRef } from "react";

interface Props {
  className?: string;
}

/**
 * Decorative fixed layer — 3 warm-gold blurred radial glows drifting slowly
 * behind all content. Required for glass surfaces to read (blur over a flat
 * colour is invisible). Purely decorative; no user-facing text.
 *
 * Performance guard: one fixed layer, no glass nested inside, animations
 * paused when the tab is hidden.
 */
export default function AmbientBackground({ className }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleVisibilityChange = () => {
      const state = document.hidden ? "paused" : "running";
      container
        .querySelectorAll<HTMLElement>("[data-ambient-blob]")
        .forEach((el) => {
          el.style.animationPlayState = state;
        });
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    handleVisibilityChange(); // sync initial play state (e.g. launched with --start-hidden)
    return () => {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, []);

  return (
    <div
      ref={containerRef}
      aria-hidden="true"
      className={className}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: -1,
        pointerEvents: "none",
        overflow: "hidden",
      }}
    >
      {/* Glow 1 — lower-left, large primary gold haze */}
      <div
        data-ambient-blob=""
        className="ambient-blob-1"
        style={{
          position: "absolute",
          width: "60vw",
          height: "60vw",
          bottom: "-20%",
          left: "-10%",
          background:
            "radial-gradient(ellipse at center, rgba(224,165,63,0.18) 0%, rgba(169,118,15,0.08) 50%, transparent 70%)",
          filter: "blur(60px)",
        }}
      />
      {/* Glow 2 — upper-right, warm specular amber */}
      <div
        data-ambient-blob=""
        className="ambient-blob-2"
        style={{
          position: "absolute",
          width: "50vw",
          height: "50vw",
          top: "-15%",
          right: "-5%",
          background:
            "radial-gradient(ellipse at center, rgba(251,231,161,0.12) 0%, rgba(224,165,63,0.06) 55%, transparent 75%)",
          filter: "blur(70px)",
        }}
      />
      {/* Glow 3 — centre, deep pressed-gold depth */}
      <div
        data-ambient-blob=""
        className="ambient-blob-3"
        style={{
          position: "absolute",
          width: "40vw",
          height: "40vw",
          top: "30%",
          left: "30%",
          background:
            "radial-gradient(ellipse at center, rgba(198,138,46,0.10) 0%, rgba(169,118,15,0.04) 60%, transparent 80%)",
          filter: "blur(80px)",
        }}
      />
    </div>
  );
}
