import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, events } from "@/bindings";

/** Compact display for tile values: 1284 -> "1.3K". */
const compact = (n: number) =>
  new Intl.NumberFormat(undefined, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(n);

type Stats = { dictations: number; words: number };

const PAGE = 200;
// ponytail: cap the aggregation at 2000 entries (10 pages) — stats become
// "over your recent history"; add a backend COUNT/SUM command if exactness
// ever matters.
const MAX_ENTRIES = 2000;

/** M9 usage insights: frontend aggregation over the history DB. WPM is
 * omitted — history entries carry no duration field. */
export const Insights: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const [stats, setStats] = useState<Stats | null>(null);
  // Recompute when the pipeline adds entries while the pane is open.
  const [refreshNonce, setRefreshNonce] = useState(0);

  useEffect(() => {
    const unlisten = events.historyUpdatePayload.listen(() => {
      setRefreshNonce((n) => n + 1);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      let dictations = 0;
      let words = 0;
      let cursor: number | null = null;
      for (let fetched = 0; fetched < MAX_ENTRIES; ) {
        const result = await commands.getHistoryEntries(cursor, PAGE);
        if (result.status !== "ok" || cancelled) return;
        const { entries, has_more } = result.data;
        for (const entry of entries) {
          const text = entry.post_processed_text ?? entry.transcription_text;
          dictations += 1;
          words += text.split(/\s+/).filter(Boolean).length;
        }
        fetched += entries.length;
        if (!has_more || entries.length === 0) break;
        cursor = entries[entries.length - 1].id;
      }
      if (!cancelled) setStats({ dictations, words });
    })().catch((error) => {
      console.error("Failed to compute insights:", error);
    });
    return () => {
      cancelled = true;
    };
  }, [refreshNonce]);

  if (!stats || stats.dictations === 0) return null;

  const tiles = [
    { label: t("insights.dictations"), value: stats.dictations },
    { label: t("insights.words"), value: stats.words },
    {
      label: t("insights.wordsPerDictation"),
      value: Math.round(stats.words / stats.dictations),
    },
  ];

  return (
    <div className="grid grid-cols-3 gap-2">
      {tiles.map((tile) => (
        <div
          key={tile.label}
          className="bg-background border border-border rounded-lg px-4 py-3"
        >
          <p className="text-xs text-text/60">{tile.label}</p>
          <p className="text-xl font-semibold text-text/90">
            {compact(tile.value)}
          </p>
        </div>
      ))}
    </div>
  );
});

Insights.displayName = "Insights";
