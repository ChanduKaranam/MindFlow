import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/Button";

const Scratchpad: React.FC = () => {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="flex flex-col h-screen bg-background text-text">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <span className="text-sm font-medium">{t("scratchpad.title")}</span>
        <div className="flex items-center gap-2">
          <Button variant="secondary" size="sm" onClick={handleCopy}>
            {copied ? t("scratchpad.copied") : t("scratchpad.copy")}
          </Button>
          <Button variant="secondary" size="sm" onClick={() => setText("")}>
            {t("scratchpad.clear")}
          </Button>
        </div>
      </div>
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder={t("scratchpad.placeholder")}
        autoFocus
        className="flex-1 w-full resize-none bg-transparent p-3 text-sm focus:outline-none placeholder:text-text-secondary"
      />
    </div>
  );
};

export default Scratchpad;
