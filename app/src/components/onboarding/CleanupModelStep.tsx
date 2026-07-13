import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { ModelInfo } from "@/bindings";
import ModelCard from "./ModelCard";
import OnboardingStepper from "./OnboardingStepper";
import AmbientBackground from "../shared/AmbientBackground";
import { useModelStore } from "../../stores/modelStore";

interface Props {
  onDone: () => void;
  stepIndex: number;
  stepTotal: number;
}

// Optional AI-cleanup model download, offered right after the main STT model
// step. Downloading is fire-and-forget: unlike Onboarding.tsx's main model
// flow, there's no setActiveModel call here — a downloaded TextLlm model is
// picked up automatically by the cleanup pipeline (see AiCleanup.tsx's
// "auto" option), it doesn't replace the active transcription model.
const CleanupModelStep: React.FC<Props> = ({
  onDone,
  stepIndex,
  stepTotal,
}) => {
  const { t } = useTranslation();
  const {
    models,
    downloadModel,
    downloadingModels,
    verifyingModels,
    downloadProgress,
    downloadStats,
  } = useModelStore();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [tier, setTier] = useState<string | null>(null);

  useEffect(() => {
    invoke<string>("recommended_tier_cmd")
      .then(setTier)
      .catch(() => {
        // Non-fatal: tier badge simply won't show
      });
  }, []);

  // Watch for the selected model to finish downloading + verifying. TextLlm
  // entries are single-file GGUFs, so extraction never applies — but we keep
  // the guard shape consistent with Onboarding.tsx's watcher regardless.
  useEffect(() => {
    if (!selectedId) return;
    const model = models.find((m) => m.id === selectedId);
    const stillDownloading = selectedId in downloadingModels;
    const stillVerifying = selectedId in verifyingModels;
    if (model?.is_downloaded && !stillDownloading && !stillVerifying) {
      onDone();
    }
  }, [selectedId, models, downloadingModels, verifyingModels, onDone]);

  const handleDownload = (modelId: string) => {
    setSelectedId(modelId);
    void downloadModel(modelId);
  };

  const llmModels = models.filter(
    (m: ModelInfo) => m.engine_type === "TextLlm" && !m.is_downloaded,
  );
  const recommended = llmModels.find((m) => m.tier === tier) ?? llmModels[0];

  return (
    <div className="relative min-h-screen flex items-center justify-center p-6">
      <AmbientBackground />
      <div
        className="glass rounded-2xl p-8 w-full flex flex-col gap-6"
        style={{ maxWidth: "560px" }}
      >
        <OnboardingStepper current={stepIndex} total={stepTotal} />
        <h2 className="text-center font-medium">
          {t("onboarding.cleanup.title")}
        </h2>
        <p className="text-text-secondary text-center">
          {t("onboarding.cleanup.subtitle")}
        </p>
        {recommended && (
          <ModelCard
            model={recommended}
            variant="featured"
            status={
              recommended.id in downloadingModels
                ? "downloading"
                : recommended.id in verifyingModels
                  ? "verifying"
                  : "downloadable"
            }
            disabled={selectedId !== null}
            onSelect={handleDownload}
            onDownload={handleDownload}
            downloadProgress={downloadProgress[recommended.id]?.percentage}
            downloadSpeed={downloadStats[recommended.id]?.speed}
          />
        )}
        <button
          className="text-text-secondary underline"
          onClick={onDone}
          disabled={selectedId !== null}
        >
          {t("onboarding.cleanup.skip")}
        </button>
      </div>
    </div>
  );
};

export default CleanupModelStep;
