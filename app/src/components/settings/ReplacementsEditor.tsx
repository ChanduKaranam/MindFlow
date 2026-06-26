import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { SettingContainer } from "../ui/SettingContainer";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { useSettings } from "../../hooks/useSettings";
import type { Replacement } from "@/bindings";

interface ReplacementsEditorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const ReplacementsEditor: React.FC<ReplacementsEditorProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const replacements = (getSetting("replacements") ?? []) as Replacement[];
    const [from, setFrom] = useState("");
    const [to, setTo] = useState("");

    const sanitize = (s: string) => s.replace(/[<>"'&]/g, "");

    const handleAdd = () => {
      const f = sanitize(from.trim());
      const tt = sanitize(to.trim());
      if (!f || f.length > 100 || tt.length > 200) return;
      if (replacements.some((r) => r.from.toLowerCase() === f.toLowerCase())) {
        toast.error(
          t("settings.advanced.replacements.duplicate", { word: f }),
        );
        return;
      }
      updateSetting("replacements", [...replacements, { from: f, to: tt }]);
      setFrom("");
      setTo("");
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAdd();
      }
    };

    const handleRemove = (index: number) => {
      updateSetting(
        "replacements",
        replacements.filter((_, i) => i !== index),
      );
    };

    return (
      <SettingContainer
        title={t("settings.advanced.replacements.title")}
        description={t("settings.advanced.replacements.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        layout="stacked"
      >
        <div className="flex flex-col gap-2 w-full">
          <div className="flex gap-2">
            <Input
              type="text"
              value={from}
              onChange={(e) => setFrom(e.target.value)}
              placeholder={t(
                "settings.advanced.replacements.fromPlaceholder",
              )}
              variant="compact"
              disabled={isUpdating("replacements")}
            />
            <Input
              type="text"
              value={to}
              onChange={(e) => setTo(e.target.value)}
              placeholder={t("settings.advanced.replacements.toPlaceholder")}
              variant="compact"
              disabled={isUpdating("replacements")}
              onKeyDown={handleKeyDown}
            />
            <Button
              onClick={handleAdd}
              disabled={!from.trim() || isUpdating("replacements")}
              variant="primary"
              size="md"
            >
              {t("settings.advanced.replacements.add")}
            </Button>
          </div>
          {replacements.length > 0 && (
            <div className="flex flex-col gap-1">
              {replacements.map((r, i) => (
                <div
                  key={`${r.from}-${i}`}
                  className={`flex items-center justify-between px-2 py-1 rounded ${grouped ? "" : "border border-mid-gray/20"} bg-mid-gray/10`}
                >
                  <span className="text-sm">
                    {`${r.from} → ${r.to || "∅"}`}
                  </span>
                  <Button
                    onClick={() => handleRemove(i)}
                    aria-label={t("settings.advanced.replacements.remove")}
                    variant="secondary"
                    size="sm"
                    disabled={isUpdating("replacements")}
                  >
                    <svg
                      className="w-3 h-3"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M6 18L18 6M6 6l12 12"
                      />
                    </svg>
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>
      </SettingContainer>
    );
  },
);
