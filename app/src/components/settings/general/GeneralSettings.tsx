import React from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { MicrophoneSelector } from "../MicrophoneSelector";
import { ShortcutInput } from "../ShortcutInput";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { OutputDeviceSelector } from "../OutputDeviceSelector";
import { RecordingModeSelector } from "../RecordingModeSelector";
import { type RecordingMode } from "@/bindings";
import { AudioFeedback } from "../AudioFeedback";
import { useSettings } from "../../../hooks/useSettings";
import { VolumeSlider } from "../VolumeSlider";
import { MuteWhileRecording } from "../MuteWhileRecording";
import { SpokenCommands } from "../SpokenCommands";
import { MicSensitivitySlider } from "../MicSensitivitySlider";
import { NoiseSuppression } from "../NoiseSuppression";
import { ModelSettingsCard } from "./ModelSettingsCard";

export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { audioFeedbackEnabled, getSetting } = useSettings();
  const recordingMode = getSetting("recording_mode") as RecordingMode | undefined;
  const isLinux = type() === "linux";
  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.general.title")}>
        <ShortcutInput shortcutId="transcribe" grouped={true} />
        <RecordingModeSelector descriptionMode="tooltip" grouped={true} />
        {/* Cancel shortcut is hidden in Hold mode (release key stops) and on Linux (dynamic shortcut instability) */}
        {!isLinux && recordingMode !== "hold" && (
          <ShortcutInput shortcutId="cancel" grouped={true} />
        )}
        <SpokenCommands descriptionMode="tooltip" grouped={true} />
      </SettingsGroup>
      <ModelSettingsCard />
      <SettingsGroup title={t("settings.sound.title")}>
        <MicrophoneSelector descriptionMode="tooltip" grouped={true} />
        <MuteWhileRecording descriptionMode="tooltip" grouped={true} />
        <MicSensitivitySlider />
        <NoiseSuppression descriptionMode="tooltip" grouped={true} />
        <AudioFeedback descriptionMode="tooltip" grouped={true} />
        <OutputDeviceSelector
          descriptionMode="tooltip"
          grouped={true}
          disabled={!audioFeedbackEnabled}
        />
        <VolumeSlider disabled={!audioFeedbackEnabled} />
      </SettingsGroup>
    </div>
  );
};
