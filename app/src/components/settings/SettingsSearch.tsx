import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import type { SidebarSection } from "../Sidebar";

interface Props {
  /** Jump to a settings section (sets the active sidebar section). */
  onJump: (section: SidebarSection) => void;
}

interface IndexEntry {
  /** i18n suffix under settings.search.items.* for the display title. */
  id: string;
  section: SidebarSection;
  /** Lower-case English match terms (matched in addition to the title). */
  keywords: string[];
}

/**
 * Static index of the headline settings plus the M1-M4 feature toggles, each
 * mapped to the sidebar section that renders it. Titles are translated at
 * search time; keywords stay English so a user typing "denoise" or "ptt"
 * still finds the setting regardless of UI language.
 */
const SEARCH_INDEX: IndexEntry[] = [
  {
    id: "hotkey",
    section: "general",
    keywords: ["shortcut", "hotkey", "key", "trigger", "binding", "record"],
  },
  {
    id: "microphone",
    section: "general",
    keywords: ["microphone", "mic", "input", "device", "audio"],
  },
  {
    id: "handsFree",
    section: "general",
    keywords: [
      "recording mode",
      "hands free",
      "hands-free",
      "push to talk",
      "ptt",
      "hold",
      "toggle",
      "enter to stop",
    ],
  },
  {
    id: "spokenCommands",
    section: "general",
    keywords: ["spoken commands", "voice commands", "new line", "commands"],
  },
  {
    id: "noiseSuppression",
    section: "general",
    keywords: ["noise", "suppression", "denoise", "background", "gtcrn"],
  },
  {
    id: "language",
    section: "general",
    keywords: ["language", "translate", "translation", "locale", "english"],
  },
  {
    id: "dictionary",
    section: "advanced",
    keywords: [
      "dictionary",
      "custom words",
      "vocabulary",
      "spelling",
      "names",
      "words",
    ],
  },
  {
    id: "replacements",
    section: "advanced",
    keywords: ["replacements", "replace", "substitution", "find replace"],
  },
  {
    id: "snippets",
    section: "advanced",
    keywords: ["snippets", "expansion", "macro", "abbreviation"],
  },
  {
    id: "model",
    section: "models",
    keywords: ["model", "whisper", "parakeet", "accuracy", "download", "speech"],
  },
];

export const SettingsSearch: React.FC<Props> = ({ onJump }) => {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return SEARCH_INDEX.filter((entry) => {
      const title = t(`settings.search.items.${entry.id}`).toLowerCase();
      if (title.includes(q)) return true;
      return entry.keywords.some((k) => k.includes(q));
    });
  }, [query, t]);

  // Dismiss the dropdown on any click outside the search box.
  useEffect(() => {
    const handleOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handleOutside);
    return () => document.removeEventListener("mousedown", handleOutside);
  }, []);

  const handleSelect = (entry: IndexEntry) => {
    onJump(entry.section);
    setQuery("");
    setOpen(false);
  };

  const showDropdown = open && query.trim().length > 0;

  return (
    <div ref={containerRef} className="relative w-full px-2 pt-2">
      <div className="relative">
        <Search
          size={14}
          className="absolute start-2 top-1/2 -translate-y-1/2 text-text-secondary pointer-events-none"
          aria-hidden="true"
        />
        <input
          type="text"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setOpen(true);
          }}
          onFocus={() => query.trim() && setOpen(true)}
          placeholder={t("settings.search.placeholder")}
          aria-label={t("settings.search.placeholder")}
          className="w-full rounded-lg bg-surface-high border border-border ps-7 pe-2 py-1.5 text-sm text-text placeholder:text-text-secondary focus:outline-none focus:border-accent"
        />
      </div>

      {/* Dropdown is opaque (not glass) and wider than the narrow sidebar so it
          overflows to the right over the content — keeps result titles on one
          line and avoids the sidebar nav bleeding through behind it. */}
      {showDropdown && (
        <div
          className="absolute start-2 top-full mt-1 z-30 w-64 bg-surface border border-border shadow-xl rounded-lg overflow-y-auto overflow-x-hidden"
          style={{ maxHeight: "calc(100vh - 220px)" }}
        >
          {results.length === 0 ? (
            <p className="px-3 py-2 text-sm text-text-secondary">
              {t("settings.search.noResults")}
            </p>
          ) : (
            results.map((entry) => (
              <button
                key={entry.id}
                type="button"
                onClick={() => handleSelect(entry)}
                className="w-full min-w-0 text-start px-3 py-2 hover:bg-surface-high transition-colors cursor-pointer flex flex-col gap-0.5"
              >
                <span className="block truncate text-sm text-text font-medium">
                  {t(`settings.search.items.${entry.id}`)}
                </span>
                <span className="block truncate text-xs text-text-secondary">
                  {t("settings.search.inSection", {
                    section: t(`sidebar.${entry.section}`),
                  })}
                </span>
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
};
