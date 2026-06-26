import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import AmbientBackground from "@/components/shared/AmbientBackground";
import OnboardingStepper from "./OnboardingStepper";

interface Props {
  hotkey: string;
  onDone: () => void;
  stepIndex: number;
  stepTotal: number;
}

interface WinData {
  words: number;
  seconds: number;
  factor: number;
}

/** Matches the string payload of the backend "show-overlay" event. */
type OverlayState = "recording" | "transcribing" | "processing";

/**
 * TryItNowStep — onboarding "peak" (peak-end rule).
 *
 * The user holds their hotkey and dictates the sample sentence into this
 * focused textarea. When the transcription is pasted back by the backend,
 * the component computes a speed comparison and shows the win.
 *
 * Timing source: the "show-overlay" Tauri event with payload "recording"
 * fires the moment the backend starts capturing audio (overlay.rs:339 →
 * show_recording_overlay → emit "show-overlay" / "recording"). This is the
 * earliest reliable frontend signal we have for when the user started
 * speaking, so we use it as our timer start.
 *
 * Text-arrival detection: the textarea is focused on mount, so clipboard
 * paste (the production output path in actions.rs) lands here. We detect
 * arrival via onChange (empty → non-empty) rather than listening to
 * historyUpdatePayload, which avoids a second listener and works even if
 * the user types manually during a demo session.
 *
 * Cleanup: the show-overlay unlisten function is called in the effect
 * cleanup so no memory leaks survive unmount.
 */
export default function TryItNowStep({
  hotkey,
  onDone,
  stepIndex,
  stepTotal,
}: Props) {
  const { t } = useTranslation();

  const [textareaValue, setTextareaValue] = useState("");
  const [isRecording, setIsRecording] = useState(false);
  const [winData, setWinData] = useState<WinData | null>(null);

  /** Timestamp (ms) set when the "show-overlay" / "recording" event fires. */
  const recordingStartedAt = useRef<number | null>(null);
  /**
   * Guards the win computation so we only run it once even if the user
   * keeps typing after the transcription lands.
   */
  const hasComputedWin = useRef(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-focus the textarea on mount so dictated text pastes here.
  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  // Subscribe to "show-overlay" events to drive the recording indicator
  // and capture the precise timer start for the win computation.
  useEffect(() => {
    const unlisten = listen<OverlayState>("show-overlay", (event) => {
      if (event.payload === "recording") {
        recordingStartedAt.current = Date.now();
        setIsRecording(true);
      } else {
        // "transcribing" or "processing" — still processing, not recording
        setIsRecording(false);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newVal = e.target.value;
    setTextareaValue(newVal);

    // Compute the win exactly once, the first time non-empty text arrives.
    if (!hasComputedWin.current && newVal.trim().length > 0) {
      hasComputedWin.current = true;

      const startedAt = recordingStartedAt.current;
      // Conservative fallback (10 s) if no recording event was observed —
      // keeps the factor numerically modest and never crashes.
      const elapsedSec =
        startedAt != null ? (Date.now() - startedAt) / 1000 : 10;

      const words = newVal.trim().split(/\s+/).filter(Boolean).length;
      const seconds = Math.max(1, Math.round(elapsedSec));
      // Factor = (words / elapsed seconds) / (40 wpm ÷ 60) = wpm / 40
      // Floor at 0.0167 s (≈ 1 frame) guards divide-by-zero; clamp ≥ 1.
      const factor = Math.max(
        1,
        Math.round(words / Math.max(0.0167, elapsedSec) / (40 / 60)),
      );

      setWinData({ words, seconds, factor });
    }
  };

  return (
    <div className="relative min-h-screen flex items-center justify-center p-6">
      <AmbientBackground />

      <div
        className="glass rounded-2xl p-8 w-full flex flex-col gap-6"
        style={{ maxWidth: "520px" }}
      >
        <OnboardingStepper current={stepIndex} total={stepTotal} />

        {/* Heading */}
        <h1 className="font-serif text-3xl font-light text-text leading-tight text-center">
          {t("onboarding.tryit.title")}
        </h1>

        {/* Instruction */}
        <p className="text-text-secondary text-base text-center">
          {t("onboarding.tryit.prompt", { hotkey })}
        </p>

        {/* Sample sentence — styled callout the user reads aloud */}
        <div className="rounded-lg px-4 py-3 text-sm text-center italic bg-recording/10 border border-recording/30 text-recording">
          {t("onboarding.tryit.sample")}
        </div>

        {/* Recording indicator — visible only while the backend is recording.
            Fixed height preserves layout so there's no jump when it appears. */}
        <div
          aria-live="polite"
          className="flex items-center justify-center gap-2 h-5"
        >
          {isRecording && (
            <>
              <span
                className="inline-block w-2 h-2 rounded-full bg-recording animate-pulse"
                aria-hidden="true"
              />
              <span className="text-recording text-sm font-medium">
                {t("onboarding.tryit.listening")}
              </span>
            </>
          )}
        </div>

        {/* Practice field — focused so clipboard paste lands here */}
        <textarea
          ref={textareaRef}
          value={textareaValue}
          onChange={handleChange}
          rows={4}
          aria-label={t("onboarding.tryit.title")}
          className="w-full rounded-lg px-4 py-3 text-sm text-text resize-none focus:outline-none bg-surface border border-border"
        />

        {/* Quantified win — appears once transcription is detected */}
        {winData !== null && (
          <p
            aria-live="polite"
            className="text-recording text-sm font-medium text-center"
          >
            {t("onboarding.tryit.win", {
              words: winData.words,
              seconds: winData.seconds,
              factor: winData.factor,
            })}
          </p>
        )}

        {/* Primary CTA — always shown so users who skip dictation can proceed */}
        <button
          type="button"
          onClick={onDone}
          className="btn-gold sheen w-full rounded-lg px-6 py-3 text-base font-semibold cursor-pointer"
        >
          {t("onboarding.tryit.continue")}
        </button>
      </div>

      {/* Skip — outside the card for Fitts distance, low-emphasis */}
      <div className="absolute bottom-8 inset-x-0 flex justify-center">
        <button
          type="button"
          onClick={onDone}
          className="text-text-secondary text-sm underline-offset-2 hover:underline"
        >
          {t("onboarding.tryit.skip")}
        </button>
      </div>
    </div>
  );
}
