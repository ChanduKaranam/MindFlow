import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { useSettings } from "../../../hooks/useSettings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingsGroup } from "../../ui/SettingsGroup";

/** M9 privacy-safe context: independent, default-OFF toggles for what the
 * cleanup prompt may see. Everything stays on-device; the "context-used"
 * event is the audit trail showing which sources the last dictation used. */
export const ContextSettings: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [lastUsed, setLastUsed] = useState<string[]>([]);

  useEffect(() => {
    const unlisten = listen<string[]>("context-used", (event) => {
      setLastUsed(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <SettingsGroup title={t("contextAwareness.title")}>
      <div className="px-4 pt-3 text-xs text-text/60">
        {t("contextAwareness.description")}
      </div>
      <ToggleSwitch
        checked={getSetting("context_window_title") ?? false}
        onChange={(v) => updateSetting("context_window_title", v)}
        isUpdating={isUpdating("context_window_title")}
        label={t("contextAwareness.windowTitle")}
        description={t("contextAwareness.windowTitleDescription")}
        descriptionMode="tooltip"
        grouped
      />
      <ToggleSwitch
        checked={getSetting("context_selection") ?? false}
        onChange={(v) => updateSetting("context_selection", v)}
        isUpdating={isUpdating("context_selection")}
        label={t("contextAwareness.selection")}
        description={t("contextAwareness.selectionDescription")}
        descriptionMode="tooltip"
        grouped
      />
      <ToggleSwitch
        checked={getSetting("context_clipboard") ?? false}
        onChange={(v) => updateSetting("context_clipboard", v)}
        isUpdating={isUpdating("context_clipboard")}
        label={t("contextAwareness.clipboard")}
        description={t("contextAwareness.clipboardDescription")}
        descriptionMode="tooltip"
        grouped
      />
      {lastUsed.length > 0 && (
        <div className="px-4 py-2 flex items-center gap-2 flex-wrap text-xs text-text/60">
          <span>{t("contextAwareness.lastUsed")}</span>
          {lastUsed.map((source) => (
            <span
              key={source}
              className="px-1.5 py-0.5 rounded bg-surface-high border border-border text-text/80"
            >
              {t(`contextAwareness.source.${source}`, {
                defaultValue: source,
              })}
            </span>
          ))}
        </div>
      )}
    </SettingsGroup>
  );
});

ContextSettings.displayName = "ContextSettings";
