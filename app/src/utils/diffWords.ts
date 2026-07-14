export type DiffPart = {
  type: "equal" | "delete" | "insert";
  text: string;
};

/**
 * Word-level diff of two strings via LCS over whitespace-split tokens.
 * Adjacent tokens of the same type are merged so the output is a short list
 * of runs suitable for rendering (deletions struck through, insertions
 * highlighted).
 */
// ponytail: O(n*m) DP table — fine for dictation-sized texts, swap for
// Myers diff if entries ever reach book length.
export function diffWords(a: string, b: string): DiffPart[] {
  const aw = a.split(/\s+/).filter(Boolean);
  const bw = b.split(/\s+/).filter(Boolean);
  const n = aw.length;
  const m = bw.length;

  // lcs[i][j] = LCS length of aw[i..] and bw[j..]
  const lcs: number[][] = Array.from({ length: n + 1 }, () =>
    new Array<number>(m + 1).fill(0),
  );
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      lcs[i][j] =
        aw[i] === bw[j]
          ? lcs[i + 1][j + 1] + 1
          : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
    }
  }

  const parts: DiffPart[] = [];
  const push = (type: DiffPart["type"], word: string) => {
    const last = parts[parts.length - 1];
    if (last && last.type === type) {
      last.text += ` ${word}`;
    } else {
      parts.push({ type, text: word });
    }
  };

  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (aw[i] === bw[j]) {
      push("equal", aw[i]);
      i++;
      j++;
    } else if (lcs[i + 1][j] >= lcs[i][j + 1]) {
      push("delete", aw[i]);
      i++;
    } else {
      push("insert", bw[j]);
      j++;
    }
  }
  for (; i < n; i++) push("delete", aw[i]);
  for (; j < m; j++) push("insert", bw[j]);

  return parts;
}
