import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import { useModelStore } from "../../../stores/modelStore";
import { commands, type ModelInfo } from "@/bindings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { Dropdown, type DropdownOption } from "../../ui/Dropdown";
import { Button } from "../../ui/Button";
import { Alert } from "../../ui/Alert";

const SENSITIVITY = { off: 0, conservative: 0.18, aggressive: 0.35 } as const;
type SensitivityKey = keyof typeof SENSITIVITY;

/** AI cleanup settings card: master toggle, per-behavior flags, cleanup
 * model selection/download, and name-correction sensitivity. Mirrors the
 * SettingsGroup + ToggleSwitch/SettingContainer idiom used by every other
 * settings card (see SpokenCommands, PostProcessingSettings). */
export const AiCleanup: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const { models, downloadModel, downloadingModels, downloadProgress } =
    useModelStore();
  const [recommendedTier, setRecommendedTier] = useState<string | null>(null);

  useEffect(() => {
    commands.recommendedTierCmd().then((result) => {
      if (result.status === "ok") setRecommendedTier(result.data);
    });
  }, []);

  const enabled = getSetting("ai_cleanup_enabled") ?? true;
  const modelId = getSetting("cleanup_model_id") ?? null;
  const threshold = getSetting("word_correction_threshold") ?? 0.18;

  const llmModels = models.filter(
    (m: ModelInfo) => m.engine_type === "TextLlm",
  );
  // Mirrors the backend's resolve_model_id: explicit selection (or the tier
  // default in Auto mode); when that isn't downloaded, cleanup runs on the
  // largest downloaded text-LLM instead of silently doing nothing.
  const preferredModel = modelId
    ? llmModels.find((m) => m.id === modelId)
    : llmModels.find((m) => m.tier && m.tier === recommendedTier);
  const fallbackModel = llmModels
    .filter((m) => m.is_downloaded)
    .sort((a, b) => Number(b.size_mb) - Number(a.size_mb))[0];
  const undownloadedModels = llmModels.filter((m) => !m.is_downloaded);

  const sensitivity: SensitivityKey =
    threshold === 0 ? "off" : threshold > 0.25 ? "aggressive" : "conservative";

  const modelOptions: DropdownOption[] = [
    { value: "", label: t("aiCleanup.modelAuto") },
    ...llmModels.map((m) => ({
      value: m.id,
      label: `${m.name} (${Number(m.size_mb)} MB)${
        m.tier && m.tier === recommendedTier
          ? t("aiCleanup.recommendedSuffix")
          : ""
      }`,
    })),
  ];

  const sensitivityOptions: DropdownOption[] = [
    { value: "off", label: t("aiCleanup.sensitivityOff") },
    { value: "conservative", label: t("aiCleanup.sensitivityConservative") },
    { value: "aggressive", label: t("aiCleanup.sensitivityAggressive") },
  ];

  return (
    <SettingsGroup title={t("aiCleanup.title")}>
      <ToggleSwitch
        checked={enabled}
        onChange={(v) => updateSetting("ai_cleanup_enabled", v)}
        isUpdating={isUpdating("ai_cleanup_enabled")}
        label={t("aiCleanup.master")}
        description={t("aiCleanup.description")}
        descriptionMode="tooltip"
        grouped
      />
      {enabled && (
        <>
          <ToggleSwitch
            checked={getSetting("cleanup_smart") ?? true}
            onChange={(v) => updateSetting("cleanup_smart", v)}
            isUpdating={isUpdating("cleanup_smart")}
            label={t("aiCleanup.smart")}
            description={t("aiCleanup.smartDescription")}
            descriptionMode="tooltip"
            grouped
          />
          <ToggleSwitch
            checked={getSetting("cleanup_self_correction") ?? true}
            onChange={(v) => updateSetting("cleanup_self_correction", v)}
            isUpdating={isUpdating("cleanup_self_correction")}
            label={t("aiCleanup.selfCorrection")}
            description={t("aiCleanup.selfCorrectionDescription")}
            descriptionMode="tooltip"
            grouped
          />
          <ToggleSwitch
            checked={getSetting("cleanup_preserve_technical") ?? true}
            onChange={(v) => updateSetting("cleanup_preserve_technical", v)}
            isUpdating={isUpdating("cleanup_preserve_technical")}
            label={t("aiCleanup.preserveTechnical")}
            description={t("aiCleanup.preserveTechnicalDescription")}
            descriptionMode="tooltip"
            grouped
          />

          <SettingContainer
            title={t("aiCleanup.model")}
            description={t("aiCleanup.modelDescription")}
            descriptionMode="tooltip"
            layout="stacked"
            grouped
          >
            <div className="space-y-2">
              <Dropdown
                options={modelOptions}
                selectedValue={modelId ?? ""}
                onSelect={(value) =>
                  updateSetting("cleanup_model_id", value || null)
                }
                disabled={isUpdating("cleanup_model_id")}
              />
              {undownloadedModels.length > 0 && (
                <div className="flex flex-wrap gap-2">
                  {undownloadedModels.map((m) => {
                    const isDownloading = m.id in downloadingModels;
                    const percent = Math.round(
                      downloadProgress[m.id]?.percentage ?? 0,
                    );
                    return (
                      <Button
                        key={m.id}
                        variant="secondary"
                        size="sm"
                        disabled={isDownloading}
                        onClick={() => downloadModel(m.id)}
                      >
                        {isDownloading
                          ? t("aiCleanup.downloading", { percent })
                          : `${t("aiCleanup.download")} ${m.name}`}
                      </Button>
                    );
                  })}
                </div>
              )}
              {preferredModel && !preferredModel.is_downloaded && (
                <Alert variant="warning" contained>
                  {fallbackModel
                    ? t("aiCleanup.fallbackModel", {
                        selected: preferredModel.name,
                        fallback: fallbackModel.name,
                      })
                    : t("aiCleanup.notDownloaded")}
                </Alert>
              )}
            </div>
          </SettingContainer>

          <SettingContainer
            title={t("aiCleanup.sensitivity")}
            description={t("aiCleanup.sensitivityDescription")}
            descriptionMode="tooltip"
            grouped
          >
            <Dropdown
              options={sensitivityOptions}
              selectedValue={sensitivity}
              onSelect={(value) =>
                updateSetting(
                  "word_correction_threshold",
                  SENSITIVITY[value as SensitivityKey],
                )
              }
              disabled={isUpdating("word_correction_threshold")}
              className="min-w-[200px]"
            />
          </SettingContainer>
        </>
      )}
    </SettingsGroup>
  );
});

AiCleanup.displayName = "AiCleanup";
