import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { invoke } from "@tauri-apps/api/core";
import type { ModelInfo } from "@/bindings";
import type { ModelCardStatus } from "./ModelCard";
import ModelCard from "./ModelCard";
import MindFlowLogo from "../icons/MindFlowLogo";
import { useModelStore } from "../../stores/modelStore";

interface OnboardingProps {
  onModelSelected: () => void;
}

const Onboarding: React.FC<OnboardingProps> = ({ onModelSelected }) => {
  const { t } = useTranslation();
  const {
    models,
    downloadModel,
    selectModel,
    downloadingModels,
    verifyingModels,
    extractingModels,
    downloadProgress,
    downloadStats,
  } = useModelStore();
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [recommendedTier, setRecommendedTier] = useState<string | null>(null);

  const isDownloading = selectedModelId !== null;

  useEffect(() => {
    invoke<string>("recommended_tier_cmd")
      .then((tier) => setRecommendedTier(tier))
      .catch(() => {
        // Non-fatal: tier badge simply won't show
      });
  }, []);

  // Watch for the selected model to finish downloading + verifying + extracting
  useEffect(() => {
    if (!selectedModelId) return;

    const model = models.find((m) => m.id === selectedModelId);
    const stillDownloading = selectedModelId in downloadingModels;
    const stillVerifying = selectedModelId in verifyingModels;
    const stillExtracting = selectedModelId in extractingModels;

    if (
      model?.is_downloaded &&
      !stillDownloading &&
      !stillVerifying &&
      !stillExtracting
    ) {
      // Model is ready — select it and transition
      selectModel(selectedModelId).then((success) => {
        if (success) {
          onModelSelected();
        } else {
          toast.error(t("onboarding.errors.selectModel"));
          setSelectedModelId(null);
        }
      });
    }
  }, [
    selectedModelId,
    models,
    downloadingModels,
    verifyingModels,
    extractingModels,
    selectModel,
    onModelSelected,
  ]);

  const handleDownloadModel = async (modelId: string) => {
    setSelectedModelId(modelId);

    // Error toast is handled centrally by the model-download-failed event listener
    // in modelStore — no toast here to avoid duplicates.
    const success = await downloadModel(modelId);
    if (!success) {
      setSelectedModelId(null);
    }
  };

  const getModelStatus = (modelId: string): ModelCardStatus => {
    if (modelId in extractingModels) return "extracting";
    if (modelId in verifyingModels) return "verifying";
    if (modelId in downloadingModels) return "downloading";
    return "downloadable";
  };

  const getModelDownloadProgress = (modelId: string): number | undefined => {
    return downloadProgress[modelId]?.percentage;
  };

  const getModelDownloadSpeed = (modelId: string): number | undefined => {
    return downloadStats[modelId]?.speed;
  };

  return (
    <div className="h-screen w-screen flex flex-col p-6 gap-4 inset-0">
      <div className="flex flex-col items-center gap-2 shrink-0">
        <MindFlowLogo width={200} />
        <p className="text-text/70 max-w-md font-medium mx-auto">
          {t("onboarding.subtitle")}
        </p>
      </div>

      <div className="max-w-[600px] w-full mx-auto text-center flex-1 flex flex-col min-h-0">
        <div className="flex flex-col gap-4 pb-6">
          {models
            .filter((m: ModelInfo) => !m.is_downloaded)
            .filter((model: ModelInfo) => model.is_recommended)
            .map((model: ModelInfo) => {
              const isRecommendedForCpu =
                recommendedTier !== null &&
                (model as { tier?: string }).tier === recommendedTier;
              return (
                <div key={model.id} className="relative">
                  {isRecommendedForCpu && (
                    <div className="text-xs text-logo-primary font-medium mb-1 text-start">
                      {t("onboarding.recommendedForYourPc")}
                    </div>
                  )}
                  <ModelCard
                    model={model}
                    variant={isRecommendedForCpu ? "featured" : "default"}
                    status={getModelStatus(model.id)}
                    disabled={isDownloading}
                    onSelect={handleDownloadModel}
                    onDownload={handleDownloadModel}
                    downloadProgress={getModelDownloadProgress(model.id)}
                    downloadSpeed={getModelDownloadSpeed(model.id)}
                  />
                </div>
              );
            })}

          {models
            .filter((m: ModelInfo) => !m.is_downloaded)
            .filter((model: ModelInfo) => !model.is_recommended)
            .sort(
              (a: ModelInfo, b: ModelInfo) =>
                Number(a.size_mb) - Number(b.size_mb),
            )
            .map((model: ModelInfo) => {
              const isRecommendedForCpu =
                recommendedTier !== null &&
                (model as { tier?: string }).tier === recommendedTier;
              return (
                <div key={model.id} className="relative">
                  {isRecommendedForCpu && (
                    <div className="text-xs text-logo-primary font-medium mb-1 text-start">
                      {t("onboarding.recommendedForYourPc")}
                    </div>
                  )}
                  <ModelCard
                    model={model}
                    variant={isRecommendedForCpu ? "featured" : "default"}
                    status={getModelStatus(model.id)}
                    disabled={isDownloading}
                    onSelect={handleDownloadModel}
                    onDownload={handleDownloadModel}
                    downloadProgress={getModelDownloadProgress(model.id)}
                    downloadSpeed={getModelDownloadSpeed(model.id)}
                  />
                </div>
              );
            })}
        </div>
      </div>
    </div>
  );
};

export default Onboarding;
