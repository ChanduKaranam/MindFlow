import React from "react";
import { useTranslation } from "react-i18next";
import { Slider } from "../ui/Slider";
import { useSettings } from "../../hooks/useSettings";

export const MicSensitivitySlider: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const vadThreshold = getSetting("vad_threshold") ?? 0.4;
  const sensitivity = 1 - vadThreshold;

  return (
    <Slider
      value={sensitivity}
      onChange={(value: number) =>
        updateSetting("vad_threshold", 1 - value)
      }
      min={0}
      max={1}
      step={0.05}
      label={t("settings.sound.micSensitivity.title")}
      description={t("settings.sound.micSensitivity.description")}
      descriptionMode="tooltip"
      grouped
      formatValue={(value) => `${Math.round(value * 100)}%`}
    />
  );
};
