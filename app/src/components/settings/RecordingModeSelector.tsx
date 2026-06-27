import { type FC } from "react";
import { useTranslation } from "react-i18next";
import { SettingContainer } from "../ui/SettingContainer";
import { Dropdown, type DropdownOption } from "../ui/Dropdown";
import { useSettings } from "../../hooks/useSettings";
import type { RecordingMode } from "@/bindings";

interface RecordingModeSelectorProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const RecordingModeSelector: FC<RecordingModeSelectorProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const currentMode = (getSetting("recording_mode") ?? "hold") as RecordingMode;

  const options: DropdownOption[] = [
    {
      value: "hold",
      label: t("settings.general.recordingMode.hold"),
    },
    {
      value: "toggle",
      label: t("settings.general.recordingMode.toggle"),
    },
    {
      value: "hands_free",
      label: t("settings.general.recordingMode.handsFree"),
    },
  ];

  return (
    <SettingContainer
      title={t("settings.general.recordingMode.title")}
      description={t("settings.general.recordingMode.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      layout="horizontal"
    >
      <Dropdown
        options={options}
        selectedValue={currentMode}
        onSelect={(value) => updateSetting("recording_mode", value as RecordingMode)}
        disabled={isUpdating("recording_mode")}
      />
    </SettingContainer>
  );
};
