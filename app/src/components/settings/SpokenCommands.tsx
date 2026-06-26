import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface SpokenCommandsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const SpokenCommands: React.FC<SpokenCommandsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("spoken_commands_enabled") ?? true;
    const numberConversion = getSetting("number_conversion_enabled") ?? false;

    return (
      <>
        <ToggleSwitch
          checked={enabled}
          onChange={(v) => updateSetting("spoken_commands_enabled", v)}
          isUpdating={isUpdating("spoken_commands_enabled")}
          label={t("settings.general.spokenCommands.label")}
          description={t("settings.general.spokenCommands.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
        <ToggleSwitch
          checked={numberConversion}
          onChange={(v) => updateSetting("number_conversion_enabled", v)}
          isUpdating={isUpdating("number_conversion_enabled")}
          disabled={!enabled}
          label={t("settings.general.numberConversion.label")}
          description={t("settings.general.numberConversion.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
      </>
    );
  },
);
