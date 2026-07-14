import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2 } from "lucide-react";
import type { Transform } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { SettingContainer } from "../../ui/SettingContainer";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";

/** M9 Transforms editor: named prompts Command Mode can run on the current
 * selection ("polish", "shorten", ...). The whole list is persisted on every
 * change via the transforms setting. */
export const Transforms: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const transforms = (getSetting("transforms") ?? []) as Transform[];
  const [newName, setNewName] = useState("");
  const [newPrompt, setNewPrompt] = useState("");
  const busy = isUpdating("transforms");

  const handleAdd = () => {
    const name = newName.trim();
    const prompt = newPrompt.trim();
    if (!name || !prompt) return;
    updateSetting("transforms", [
      ...transforms,
      { id: crypto.randomUUID(), name, prompt },
    ]);
    setNewName("");
    setNewPrompt("");
  };

  const handleRemove = (id: string) => {
    updateSetting(
      "transforms",
      transforms.filter((tr) => tr.id !== id),
    );
  };

  // Inline edits are committed on blur so typing doesn't write settings on
  // every keystroke; inputs are keyed by id so defaultValue stays in sync.
  const handleFieldBlur = (
    id: string,
    field: "name" | "prompt",
    value: string,
  ) => {
    const trimmed = value.trim();
    if (!trimmed) return;
    const current = transforms.find((tr) => tr.id === id);
    if (!current || current[field] === trimmed) return;
    updateSetting(
      "transforms",
      transforms.map((tr) =>
        tr.id === id ? { ...tr, [field]: trimmed } : tr,
      ),
    );
  };

  return (
    <SettingsGroup title={t("transforms.title")}>
      <SettingContainer
        title={t("transforms.listTitle")}
        description={t("transforms.description")}
        descriptionMode="tooltip"
        layout="stacked"
        grouped
      >
        <div className="flex flex-col gap-2 w-full">
          {transforms.map((tr) => (
            <div key={tr.id} className="flex gap-2 items-start">
              <Input
                type="text"
                defaultValue={tr.name}
                onBlur={(e) => handleFieldBlur(tr.id, "name", e.target.value)}
                placeholder={t("transforms.namePlaceholder")}
                variant="compact"
                disabled={busy}
                className="w-40 shrink-0"
              />
              <Input
                type="text"
                defaultValue={tr.prompt}
                onBlur={(e) => handleFieldBlur(tr.id, "prompt", e.target.value)}
                placeholder={t("transforms.promptPlaceholder")}
                variant="compact"
                disabled={busy}
                className="flex-1 min-w-0"
              />
              <Button
                onClick={() => handleRemove(tr.id)}
                aria-label={t("transforms.remove", { name: tr.name })}
                variant="secondary"
                size="sm"
                disabled={busy}
                className="shrink-0"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </Button>
            </div>
          ))}
          <div className="flex gap-2 items-start">
            <Input
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={t("transforms.namePlaceholder")}
              variant="compact"
              disabled={busy}
              className="w-40 shrink-0"
            />
            <Input
              type="text"
              value={newPrompt}
              onChange={(e) => setNewPrompt(e.target.value)}
              placeholder={t("transforms.promptPlaceholder")}
              variant="compact"
              disabled={busy}
              className="flex-1 min-w-0"
            />
            <Button
              onClick={handleAdd}
              disabled={!newName.trim() || !newPrompt.trim() || busy}
              variant="primary"
              size="sm"
              className="shrink-0"
            >
              {t("transforms.add")}
            </Button>
          </div>
          <p className="text-xs text-text/60">{t("transforms.runHint")}</p>
        </div>
      </SettingContainer>
    </SettingsGroup>
  );
});

Transforms.displayName = "Transforms";
