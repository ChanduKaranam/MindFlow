import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { platform } from "@tauri-apps/plugin-os";
import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
  requestAccessibilityPermission,
  requestMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { Mic, ShieldCheck, Check } from "lucide-react";
import { commands } from "@/bindings";
import OnboardingStepper from "./OnboardingStepper";

interface Props {
  kind: "microphone" | "accessibility";
  onGranted: () => void;
  onSkip?: () => void;
  stepIndex: number;
  stepTotal: number;
}

type GrantState = "idle" | "waiting" | "granted";

type DetectedPlatform = "macos" | "windows" | "other";

export default function PermissionPrimer({
  kind,
  onGranted,
  onSkip,
  stepIndex,
  stepTotal,
}: Props) {
  const { t } = useTranslation();
  const [grantState, setGrantState] = useState<GrantState>("idle");
  const [detectedPlatform, setDetectedPlatform] =
    useState<DetectedPlatform | null>(null);
  const pollingRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const errorCountRef = useRef<number>(0);
  const MAX_POLLING_ERRORS = 3;

  // Detect platform on mount and handle pass-through for non-applicable kinds
  useEffect(() => {
    const currentPlatform = platform();
    const resolved: DetectedPlatform =
      currentPlatform === "macos"
        ? "macos"
        : currentPlatform === "windows"
          ? "windows"
          : "other";
    setDetectedPlatform(resolved);

    // If accessibility is requested on a non-macOS platform, it's a no-op
    if (kind === "accessibility" && resolved !== "macos") {
      onGranted();
    }
  }, [kind, onGranted]);

  // Cleanup polling interval on unmount
  useEffect(() => {
    return () => {
      if (pollingRef.current) {
        clearInterval(pollingRef.current);
        pollingRef.current = null;
      }
    };
  }, []);

  const stopPolling = () => {
    if (pollingRef.current) {
      clearInterval(pollingRef.current);
      pollingRef.current = null;
    }
  };

  const startPolling = (currentPlatform: DetectedPlatform) => {
    if (pollingRef.current) return;

    pollingRef.current = setInterval(async () => {
      try {
        if (kind === "microphone") {
          if (currentPlatform === "windows") {
            const status =
              await commands.getWindowsMicrophonePermissionStatus();
            const granted =
              !status.supported || status.overall_access !== "denied";
            if (granted) {
              stopPolling();
              setGrantState("granted");
              onGranted();
            }
          } else {
            // macOS
            const granted = await checkMicrophonePermission();
            if (granted) {
              stopPolling();
              setGrantState("granted");
              onGranted();
            }
          }
        } else {
          // accessibility — macOS only (non-macOS is handled by pass-through above)
          const granted = await checkAccessibilityPermission();
          if (granted) {
            stopPolling();
            setGrantState("granted");
            onGranted();
          }
        }
        errorCountRef.current = 0;
      } catch (err) {
        console.error("PermissionPrimer: error while polling permission", err);
        errorCountRef.current += 1;
        if (errorCountRef.current >= MAX_POLLING_ERRORS) {
          stopPolling();
        }
      }
    }, 1000);
  };

  const handleAllow = async () => {
    if (!detectedPlatform) return;
    try {
      if (kind === "microphone") {
        if (detectedPlatform === "windows") {
          await commands.openMicrophonePrivacySettings();
        } else {
          await requestMicrophonePermission();
        }
      } else {
        // accessibility
        await requestAccessibilityPermission();
      }
      setGrantState("waiting");
      startPolling(detectedPlatform);
    } catch (err) {
      console.error("PermissionPrimer: failed to request permission", err);
    }
  };

  // Don't render until we know the platform
  if (detectedPlatform === null) return null;

  // Non-macOS accessibility is a pass-through — render nothing while effect fires
  if (kind === "accessibility" && detectedPlatform !== "macos") return null;

  const isMic = kind === "microphone";
  const Icon = isMic ? Mic : ShieldCheck;

  return (
    <div className="h-screen w-screen flex flex-col items-center justify-center p-6 gap-8">
      <div className="glass max-w-md w-full flex flex-col items-center gap-6 p-8 rounded-2xl">
        {/* Stepper */}
        <OnboardingStepper
          current={stepIndex}
          total={stepTotal}
          className="w-full"
        />

        {/* Icon */}
        <div className="p-4 rounded-full bg-accent/20">
          <Icon className="w-10 h-10 text-accent" />
        </div>

        {/* Title */}
        <h2 className="text-xl font-semibold text-text text-center">
          {t(`onboarding.primer.${kind}.title`)}
        </h2>

        {/* Body */}
        <p className="text-text-secondary text-sm text-center leading-relaxed">
          {t(`onboarding.primer.${kind}.body`)}
        </p>

        {/* macOS accessibility steps */}
        {kind === "accessibility" && (
          <p className="text-text-secondary text-xs text-center leading-relaxed bg-white/5 border border-border rounded-lg px-4 py-3">
            {t("onboarding.primer.accessibility.steps")}
          </p>
        )}

        {/* Primary action */}
        {grantState === "granted" ? (
          <div className="flex items-center gap-2 text-emerald-400 text-sm font-medium">
            <Check className="w-5 h-5" />
            {t("onboarding.primer.granted")}
          </div>
        ) : grantState === "waiting" ? (
          <div className="flex flex-col items-center gap-3 w-full">
            <p className="text-text-secondary text-xs text-center animate-pulse">
              {t("onboarding.permissions.waiting")}
            </p>
            {kind === "accessibility" && (
              <p className="text-text-secondary text-xs text-center leading-relaxed">
                {t("onboarding.primer.accessibility.stillOff")}
              </p>
            )}
          </div>
        ) : (
          <button
            onClick={handleAllow}
            className="btn-gold sheen w-full px-6 py-3 rounded-lg border-transparent font-medium transition-colors focus:outline-none"
          >
            {t(`onboarding.primer.${kind}.allow`)}
          </button>
        )}
      </div>

      {/* Skip link — low-emphasis, away from primary button (Fitts) */}
      {onSkip && grantState !== "granted" && (
        <button
          onClick={onSkip}
          className="text-text-secondary text-sm underline-offset-2 hover:underline mt-2"
        >
          {t("onboarding.primer.skip")}
        </button>
      )}
    </div>
  );
}
