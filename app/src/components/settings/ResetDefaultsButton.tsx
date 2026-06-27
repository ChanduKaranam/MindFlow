import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { ask } from "@tauri-apps/plugin-dialog";
import { Button } from "../ui/Button";
import { commands } from "@/bindings";
import { useSettingsStore } from "@/stores/settingsStore";

/**
 * Low-emphasis, confirm-guarded button that restores every setting to its
 * built-in default. Intentionally placed at the bottom of the About screen
 * (Fitts-distant from everyday controls) and styled as `danger-ghost` so it
 * never competes with primary actions. Models, history, and recordings live
 * outside the settings store and are not affected.
 */
export const ResetDefaultsButton: React.FC = () => {
  const { t } = useTranslation();
  const [isResetting, setIsResetting] = useState(false);
  const refreshSettings = useSettingsStore((state) => state.refreshSettings);

  const handleReset = async () => {
    const confirmed = await ask(t("settings.reset.confirm.message"), {
      title: t("settings.reset.confirm.title"),
      kind: "warning",
    });
    if (!confirmed) return;

    setIsResetting(true);
    try {
      const result = await commands.resetSettingsToDefaults();
      if (result.status === "ok") {
        // Re-pull settings from the backend (source of truth) so every
        // control on screen reflects the restored defaults immediately.
        await refreshSettings();
      } else {
        console.error("Failed to reset settings:", result.error);
      }
    } catch (error) {
      console.error("Failed to reset settings:", error);
    } finally {
      setIsResetting(false);
    }
  };

  return (
    <Button
      variant="danger-ghost"
      size="md"
      onClick={handleReset}
      disabled={isResetting}
    >
      {t("settings.reset.button")}
    </Button>
  );
};
