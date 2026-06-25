import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface NoiseSuppressionToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const NoiseSuppression: React.FC<NoiseSuppressionToggleProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("noise_suppression") ?? true;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(v) => updateSetting("noise_suppression", v)}
        isUpdating={isUpdating("noise_suppression")}
        label={t("settings.sound.noiseSuppression.title")}
        description={t("settings.sound.noiseSuppression.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
