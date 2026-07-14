import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask } from "@tauri-apps/plugin-dialog";
import { useSettings } from "../../../hooks/useSettings";
import { useModelStore } from "../../../stores/modelStore";
import { commands, type ModelInfo } from "@/bindings";
import { ModelCard, type ModelCardStatus } from "../../onboarding";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { Dropdown, type DropdownOption } from "../../ui/Dropdown";
import { Alert } from "../../ui/Alert";

const SENSITIVITY = { off: 0, conservative: 0.18, aggressive: 0.35 } as const;
type SensitivityKey = keyof typeof SENSITIVITY;

/** Flags implied by each intensity preset: [smart, self_correction,
 * preserve_technical]. Mirrors change_cleanup_intensity_setting in Rust. */
const INTENSITY_FLAGS: Record<string, [boolean, boolean, boolean]> = {
  light: [true, false, false],
  medium: [true, true, true],
  high: [true, true, true],
};

/** AI cleanup settings card: master toggle, per-behavior flags, cleanup
 * model selection/download, and name-correction sensitivity. Mirrors the
 * SettingsGroup + ToggleSwitch/SettingContainer idiom used by every other
 * settings card (see SpokenCommands, PostProcessingSettings). */
export const AiCleanup: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const {
    models,
    downloadModel,
    cancelDownload,
    deleteModel,
    downloadingModels,
    downloadProgress,
    downloadStats,
    verifyingModels,
    extractingModels,
  } = useModelStore();
  const [recommendedTier, setRecommendedTier] = useState<string | null>(null);
  // Model whose download the user started from this card list; auto-selected
  // as the cleanup model once its download finishes (Onboarding.tsx pattern).
  const [pendingSelectId, setPendingSelectId] = useState<string | null>(null);

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
  // What Auto runs on right now: the tier default if downloaded, else the
  // largest downloaded model.
  const autoResolvedModel = preferredModel?.is_downloaded
    ? preferredModel
    : fallbackModel;

  // Auto-select a model the user downloaded from this list once it lands.
  useEffect(() => {
    if (!pendingSelectId) return;
    const model = llmModels.find((m) => m.id === pendingSelectId);
    const stillBusy =
      pendingSelectId in downloadingModels ||
      pendingSelectId in verifyingModels ||
      pendingSelectId in extractingModels;
    if (stillBusy) return;
    if (model?.is_downloaded) {
      updateSetting("cleanup_model_id", pendingSelectId);
    }
    setPendingSelectId(null);
  }, [
    pendingSelectId,
    llmModels,
    downloadingModels,
    verifyingModels,
    extractingModels,
    updateSetting,
  ]);

  const getModelStatus = (m: ModelInfo): ModelCardStatus => {
    if (m.id in extractingModels) return "extracting";
    if (m.id in verifyingModels) return "verifying";
    if (m.id in downloadingModels) return "downloading";
    if (m.id === modelId) return "active";
    if (m.is_downloaded) return "available";
    return "downloadable";
  };

  const handleModelDownload = (id: string) => {
    setPendingSelectId(id);
    downloadModel(id);
  };

  const handleModelDelete = async (id: string) => {
    const model = llmModels.find((m) => m.id === id);
    const modelName = model?.name || id;
    const confirmed = await ask(
      t("settings.models.deleteConfirm", { modelName }),
      { title: t("settings.models.deleteTitle"), kind: "warning" },
    );
    if (!confirmed) return;
    try {
      await deleteModel(id);
    } catch (err) {
      console.error(`Failed to delete model ${id}:`, err);
    }
    if (id === modelId) updateSetting("cleanup_model_id", null);
  };

  const sensitivity: SensitivityKey =
    threshold === 0 ? "off" : threshold > 0.25 ? "aggressive" : "conservative";

  // The knob is a preset writer over the three flags; when the flags no
  // longer match the preset the knob last wrote, it reads "custom".
  const intensity = getSetting("cleanup_intensity") ?? "medium";
  const impliedFlags = INTENSITY_FLAGS[intensity];
  const currentFlags = [
    getSetting("cleanup_smart") ?? true,
    getSetting("cleanup_self_correction") ?? true,
    getSetting("cleanup_preserve_technical") ?? true,
  ];
  const displayedIntensity =
    impliedFlags && impliedFlags.every((f, i) => f === currentFlags[i])
      ? intensity
      : "custom";

  const intensityOptions: DropdownOption[] = [
    { value: "off", label: t("aiCleanup.intensityOff") },
    { value: "light", label: t("aiCleanup.intensityLight") },
    { value: "medium", label: t("aiCleanup.intensityMedium") },
    { value: "high", label: t("aiCleanup.intensityHigh") },
    { value: "custom", label: t("aiCleanup.intensityCustom"), disabled: true },
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
          <SettingContainer
            title={t("aiCleanup.intensity")}
            description={t("aiCleanup.intensityDescription")}
            descriptionMode="tooltip"
            grouped
          >
            <Dropdown
              options={intensityOptions}
              selectedValue={displayedIntensity}
              onSelect={(value) => updateSetting("cleanup_intensity", value)}
              disabled={isUpdating("cleanup_intensity")}
              className="min-w-[200px]"
            />
          </SettingContainer>
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
          <ToggleSwitch
            checked={getSetting("instant_paste") ?? true}
            onChange={(v) => updateSetting("instant_paste", v)}
            isUpdating={isUpdating("instant_paste")}
            label={t("aiCleanup.instantPaste")}
            description={t("aiCleanup.instantPasteDescription")}
            descriptionMode="tooltip"
            grouped
          />
          <ToggleSwitch
            checked={getSetting("app_tone_enabled") ?? true}
            onChange={(v) => updateSetting("app_tone_enabled", v)}
            isUpdating={isUpdating("app_tone_enabled")}
            label={t("aiCleanup.appTone")}
            description={t("aiCleanup.appToneDescription")}
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
            <div className="space-y-3">
              <button
                type="button"
                onClick={() => updateSetting("cleanup_model_id", null)}
                disabled={isUpdating("cleanup_model_id")}
                className={`flex items-center justify-between w-full rounded-xl border-2 px-4 py-2 text-sm text-left transition-all duration-200 ${
                  modelId === null
                    ? "border-accent/50 bg-accent/10"
                    : "border-border cursor-pointer hover:border-accent/50 hover:bg-accent/5"
                }`}
              >
                <span className="font-medium text-text">
                  {t("aiCleanup.modelAutoCard")}
                </span>
                {modelId === null && autoResolvedModel && (
                  <span className="text-xs text-text/60">
                    {t("aiCleanup.resolvesTo", {
                      name: autoResolvedModel.name,
                    })}
                  </span>
                )}
              </button>
              {llmModels.map((m) => {
                const isRecommendedForCpu =
                  recommendedTier !== null && m.tier === recommendedTier;
                const isAutoResolved =
                  modelId === null && m.id === autoResolvedModel?.id;
                return (
                  <div key={m.id}>
                    {isRecommendedForCpu && (
                      <div className="text-xs text-accent font-medium mb-1 text-start">
                        {t("onboarding.recommendedForYourPc")}
                      </div>
                    )}
                    <ModelCard
                      model={m}
                      variant={isAutoResolved ? "featured" : "default"}
                      status={getModelStatus(m)}
                      disabled={isUpdating("cleanup_model_id")}
                      onSelect={(id) => updateSetting("cleanup_model_id", id)}
                      onDownload={handleModelDownload}
                      onDelete={handleModelDelete}
                      onCancel={cancelDownload}
                      downloadProgress={downloadProgress[m.id]?.percentage}
                      downloadSpeed={downloadStats[m.id]?.speed}
                      showRecommended={false}
                    />
                  </div>
                );
              })}
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
