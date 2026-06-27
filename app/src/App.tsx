import { useEffect, useState, useRef, lazy, Suspense } from "react";
import { toast, Toaster } from "sonner";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { platform } from "@tauri-apps/plugin-os";
import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { ModelStateEvent, RecordingErrorEvent } from "./lib/types/events";
import "./App.css";
import AccessibilityPermissions from "./components/AccessibilityPermissions";
import AmbientBackground from "./components/shared/AmbientBackground";
import Footer from "./components/footer";
import Onboarding, {
  AccessibilityOnboarding,
  WelcomeStep,
  PermissionPrimer,
  TryItNowStep,
  FeatureIntro,
} from "./components/onboarding";
import { Sidebar, SidebarSection, SECTIONS_CONFIG } from "./components/Sidebar";
import { useSettings } from "./hooks/useSettings";
import { useSettingsStore } from "./stores/settingsStore";
import { commands } from "@/bindings";
import { getLanguageDirection, initializeRTL } from "@/lib/utils/rtl";

// DEV-ONLY: M1 injection harness — tree-shaken out of production builds.
const DevInject = import.meta.env.DEV
  ? lazy(() => import("./components/DevInject"))
  : null;

type OnboardingStep =
  | "welcome"
  | "microphone"
  | "accessibility"
  | "model"
  | "tryit"
  | "features"
  | "done";

const renderSettingsContent = (section: SidebarSection) => {
  const ActiveComponent =
    SECTIONS_CONFIG[section]?.component || SECTIONS_CONFIG.general.component;
  return <ActiveComponent />;
};

function App() {
  const { t, i18n } = useTranslation();
  const [onboardingStep, setOnboardingStep] = useState<OnboardingStep | null>(
    null,
  );
  // Track if this is a returning user who just needs to grant permissions
  // (vs a new user who needs full onboarding including model selection)
  const [isReturningUser, setIsReturningUser] = useState(false);
  // Platform is resolved once during onboarding status check so the step
  // sequence and progress count can branch on it without re-querying.
  const [detectedPlatform, setDetectedPlatform] = useState<string | null>(null);
  const [currentSection, setCurrentSection] =
    useState<SidebarSection>("general");
  // Bumped each time the user jumps to a section via settings search; drives a
  // brief accent ring on the arrived content (Von Restorff arrival cue).
  const [searchJumpNonce, setSearchJumpNonce] = useState(0);
  const contentHighlightRef = useRef<HTMLDivElement>(null);
  const { settings, updateSetting } = useSettings();
  const direction = getLanguageDirection(i18n.language);
  const refreshAudioDevices = useSettingsStore(
    (state) => state.refreshAudioDevices,
  );
  const refreshOutputDevices = useSettingsStore(
    (state) => state.refreshOutputDevices,
  );
  const hasCompletedPostOnboardingInit = useRef(false);

  useEffect(() => {
    checkOnboardingStatus();
  }, []);

  // Initialize RTL direction when language changes
  useEffect(() => {
    initializeRTL(i18n.language);
  }, [i18n.language]);

  // Initialize Enigo, shortcuts, and refresh audio devices when main app loads.
  // Also fire at the "tryit" step so the global hotkey is live for the
  // hands-on demo (the peak-end moment) before onboarding formally completes.
  // The ref guards against re-running on the later "features"/"done" steps.
  useEffect(() => {
    if (
      (onboardingStep === "tryit" || onboardingStep === "done") &&
      !hasCompletedPostOnboardingInit.current
    ) {
      hasCompletedPostOnboardingInit.current = true;
      Promise.all([
        commands.initializeEnigo(),
        commands.initializeShortcuts(),
      ]).catch((e) => {
        console.warn("Failed to initialize:", e);
      });
      refreshAudioDevices();
      refreshOutputDevices();
    }
  }, [onboardingStep, refreshAudioDevices, refreshOutputDevices]);

  // Briefly ring the content panel when the user arrives via settings search,
  // drawing the eye to the jumped-to section (Von Restorff). Skips the initial
  // render (nonce 0) and respects reduced motion via the CSS class itself.
  useEffect(() => {
    if (searchJumpNonce === 0) return;
    const el = contentHighlightRef.current;
    if (!el) return;
    el.classList.add("search-jump-highlight");
    const timer = setTimeout(
      () => el.classList.remove("search-jump-highlight"),
      1200,
    );
    return () => clearTimeout(timer);
  }, [searchJumpNonce]);

  // Handle keyboard shortcuts for debug mode toggle
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Check for Ctrl+Shift+D (Windows/Linux) or Cmd+Shift+D (macOS)
      const isDebugShortcut =
        event.shiftKey &&
        event.key.toLowerCase() === "d" &&
        (event.ctrlKey || event.metaKey);

      if (isDebugShortcut) {
        event.preventDefault();
        const currentDebugMode = settings?.debug_mode ?? false;
        updateSetting("debug_mode", !currentDebugMode);
      }
    };

    // Add event listener when component mounts
    document.addEventListener("keydown", handleKeyDown);

    // Cleanup event listener when component unmounts
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [settings?.debug_mode, updateSetting]);

  // Listen for recording errors from the backend and show a toast
  useEffect(() => {
    const unlisten = listen<RecordingErrorEvent>("recording-error", (event) => {
      const { error_type, detail } = event.payload;

      if (error_type === "microphone_permission_denied") {
        const currentPlatform = platform();
        const platformKey = `errors.micPermissionDenied.${currentPlatform}`;
        const description = t(platformKey, {
          defaultValue: t("errors.micPermissionDenied.generic"),
        });
        toast.error(t("errors.micPermissionDeniedTitle"), { description });
      } else if (error_type === "no_input_device") {
        toast.error(t("errors.noInputDeviceTitle"), {
          description: t("errors.noInputDevice"),
        });
      } else {
        toast.error(
          t("errors.recordingFailed", { error: detail ?? "Unknown error" }),
        );
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [t]);

  // Listen for paste failures and show a toast.
  // The technical error detail is logged to handy.log on the Rust side
  // (see actions.rs `error!("Failed to paste transcription: ...")`),
  // so we show a localized, user-friendly message here instead of the raw error.
  useEffect(() => {
    const unlisten = listen("paste-error", () => {
      toast.error(t("errors.pasteFailedTitle"), {
        description: t("errors.pasteFailed"),
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [t]);

  // Listen for model loading failures and show a toast
  useEffect(() => {
    const unlisten = listen<ModelStateEvent>("model-state-changed", (event) => {
      if (event.payload.event_type === "loading_failed") {
        toast.error(
          t("errors.modelLoadFailed", {
            model:
              event.payload.model_name || t("errors.modelLoadFailedUnknown"),
          }),
          {
            description: event.payload.error,
          },
        );
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [t]);

  const revealMainWindowForPermissions = async () => {
    try {
      await commands.showMainWindowCommand();
    } catch (e) {
      console.warn("Failed to show main window for permission onboarding:", e);
    }
  };

  const checkOnboardingStatus = async () => {
    // Resolve platform up front so the step sequence is well-defined even if a
    // later async call throws and we fall back to the welcome flow.
    const currentPlatform = platform();
    setDetectedPlatform(currentPlatform);

    try {
      // Check if they have any models available
      const result = await commands.hasAnyModelsAvailable();
      const hasModels = result.status === "ok" && result.data;

      // Has the user already completed the guided first-run flow? If so we
      // never send them back through it, even if their models were removed.
      let onboardingCompleted = false;
      try {
        const settingsResult = await commands.getAppSettings();
        if (settingsResult.status === "ok") {
          onboardingCompleted = settingsResult.data.onboarding_completed ?? false;
        }
      } catch (e) {
        console.warn("Failed to read onboarding_completed flag:", e);
      }

      if (hasModels || onboardingCompleted) {
        // Returning user - check if they need to grant permissions first
        setIsReturningUser(true);

        if (currentPlatform === "macos") {
          try {
            const [hasAccessibility, hasMicrophone] = await Promise.all([
              checkAccessibilityPermission(),
              checkMicrophonePermission(),
            ]);
            if (!hasAccessibility || !hasMicrophone) {
              await revealMainWindowForPermissions();
              setOnboardingStep("accessibility");
              return;
            }
          } catch (e) {
            console.warn("Failed to check macOS permissions:", e);
            // If we can't check, proceed to main app and let them fix it there
          }
        }

        if (currentPlatform === "windows") {
          try {
            const microphoneStatus =
              await commands.getWindowsMicrophonePermissionStatus();
            if (
              microphoneStatus.supported &&
              microphoneStatus.overall_access === "denied"
            ) {
              await revealMainWindowForPermissions();
              setOnboardingStep("accessibility");
              return;
            }
          } catch (e) {
            console.warn("Failed to check Windows microphone permissions:", e);
            // If we can't check, proceed to main app and let them fix it there
          }
        }

        setOnboardingStep("done");
      } else {
        // New user - start the full guided first-run flow
        setIsReturningUser(false);
        setOnboardingStep("welcome");
      }
    } catch (error) {
      console.error("Failed to check onboarding status:", error);
      // On hard failure, treat as a new user and start the guided flow.
      setIsReturningUser(false);
      setOnboardingStep("welcome");
    }
  };

  // ─── New-user guided flow transitions ──────────────────────────────────
  const handleWelcomeContinue = () => setOnboardingStep("microphone");

  const handleMicrophoneDone = () =>
    setOnboardingStep(detectedPlatform === "macos" ? "accessibility" : "model");

  const handleAccessibilityPrimerDone = () => setOnboardingStep("model");

  const handleTryItDone = () => setOnboardingStep("features");

  const handleFeaturesFinish = async () => {
    try {
      await commands.setOnboardingCompleted(true);
    } catch (e) {
      console.warn("Failed to persist onboarding_completed:", e);
    }
    setOnboardingStep("done");
  };

  // Returning-user permission-repair path (AccessibilityOnboarding).
  const handleAccessibilityComplete = () => {
    // Returning users already have models, skip to main app.
    // (New users reach the model step via the primer flow, not this handler.)
    setOnboardingStep(isReturningUser ? "done" : "model");
  };

  const handleModelSelected = () => {
    // Model download started — advance to the hands-on demo (peak-end moment).
    setOnboardingStep("tryit");
  };

  // Step numbering for the progress indicator. macOS has an extra
  // accessibility step; the try-it and features screens share the final slot.
  const isMacOnboarding = detectedPlatform === "macos";
  const stepTotal = isMacOnboarding ? 5 : 4;
  const modelStepIndex = isMacOnboarding ? 4 : 3;
  const finalStepIndex = isMacOnboarding ? 5 : 4;
  const transcribeHotkey = settings?.bindings?.transcribe?.current_binding ?? "";

  // Still checking onboarding status
  if (onboardingStep === null) {
    return null;
  }

  if (onboardingStep === "welcome") {
    return (
      <WelcomeStep
        onContinue={handleWelcomeContinue}
        stepIndex={1}
        stepTotal={stepTotal}
      />
    );
  }

  if (onboardingStep === "microphone") {
    return (
      <PermissionPrimer
        kind="microphone"
        onGranted={handleMicrophoneDone}
        onSkip={handleMicrophoneDone}
        stepIndex={2}
        stepTotal={stepTotal}
      />
    );
  }

  if (onboardingStep === "accessibility") {
    // Returning users repairing permissions get the combined repair screen;
    // new users get the explain-before-prompt accessibility primer.
    if (isReturningUser) {
      return (
        <AccessibilityOnboarding onComplete={handleAccessibilityComplete} />
      );
    }
    return (
      <PermissionPrimer
        kind="accessibility"
        onGranted={handleAccessibilityPrimerDone}
        onSkip={handleAccessibilityPrimerDone}
        stepIndex={3}
        stepTotal={stepTotal}
      />
    );
  }

  if (onboardingStep === "model") {
    return (
      <Onboarding
        onModelSelected={handleModelSelected}
        stepIndex={modelStepIndex}
        stepTotal={stepTotal}
      />
    );
  }

  if (onboardingStep === "tryit") {
    return (
      <TryItNowStep
        hotkey={transcribeHotkey}
        onDone={handleTryItDone}
        stepIndex={finalStepIndex}
        stepTotal={stepTotal}
      />
    );
  }

  if (onboardingStep === "features") {
    return (
      <FeatureIntro
        onFinish={handleFeaturesFinish}
        stepIndex={finalStepIndex}
        stepTotal={stepTotal}
      />
    );
  }

  return (
    <div
      dir={direction}
      className="h-screen flex flex-col select-none cursor-default"
    >
      {/* Ambient glow drifting behind the frosted sidebar + settings cards.
          Fixed + z-index:-1, so it sits behind all chrome in both themes. */}
      <AmbientBackground />
      <Toaster
        theme="system"
        toastOptions={{
          unstyled: true,
          classNames: {
            toast:
              "bg-background border border-border rounded-lg shadow-lg px-4 py-3 flex items-center gap-3 text-sm",
            title: "font-medium",
            description: "text-text-secondary",
          },
        }}
      />
      {/* DEV-ONLY: M1 injection harness */}
      {DevInject && (
        <Suspense fallback={null}>
          <DevInject />
        </Suspense>
      )}
      {/* Main content area that takes remaining space */}
      <div className="flex-1 flex overflow-hidden">
        <Sidebar
          activeSection={currentSection}
          onSectionChange={setCurrentSection}
          onSearchJump={() => setSearchJumpNonce((n) => n + 1)}
        />
        {/* Scrollable content area */}
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="flex-1 overflow-y-auto">
            <div
              ref={contentHighlightRef}
              className="flex flex-col items-center p-4 gap-4 rounded-lg"
            >
              <AccessibilityPermissions />
              {renderSettingsContent(currentSection)}
            </div>
          </div>
        </div>
      </div>
      {/* Fixed footer at bottom */}
      <Footer />
    </div>
  );
}

export default App;
