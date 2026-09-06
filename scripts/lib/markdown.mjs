// Markdown helpers shared by the doc checks, so there is one definition of each
// rather than a copy per checker.

/**
 * Blank out fenced code blocks, preserving line numbering.
 *
 * This corpus documents its own formats constantly, so a doc quoting
 * "# T99 — illustration" inside a fence must not read as a register entry, and a
 * fenced table must not be checked as a table.
 *
 * Returns `{ lines, unterminated }`. It MATCHES fences rather than counting
 * them, and it reports an unclosed one, because the earlier bare parity toggle
 * failed open in the worst possible direction: one odd marker blanked the whole
 * rest of the file, and check-structure, check-registers and check-links each
 * went on printing their unchanged ✓ counts over a corpus they could no longer
 * see. No forgotten fence was even required. A `~~~text` block quoting the act
 * of opening a ```mermaid fence is three markers — odd — and a ````markdown
 * block containing ``` inverted the toggle instead, making quoted example
 * content live. CommonMark's actual rules close both holes: a fence closes only
 * on the SAME character, at least as long as the opener, with nothing after it.
 */
export function scanFences(text) {
  // The info string is m[2]. A backtick fence may not carry a backtick in it —
  // that is CommonMark's rule, and it is what keeps a lone `` `x` `` span in
  // running prose from ever reading as an opener.
  const FENCE = /^\s*(`{3,}|~{3,})(.*)$/;
  let open = null; // { char, len, line }
  const lines = text.split("\n").map((line, i) => {
    const m = FENCE.exec(line);
    if (m) {
      const char = m[1][0];
      const len = m[1].length;
      if (!open) {
        if (!(char === "`" && m[2].includes("`"))) {
          open = { char, len, line: i + 1 };
          return "";
        }
      } else if (char === open.char && len >= open.len && m[2].trim() === "") {
        open = null;
        return "";
      }
    }
    return open ? "" : line;
  });
  return { lines, unterminated: open };
}

/** Just the blanked lines. Callers that report the unterminated case use scanFences. */
export function stripFences(text) {
  return scanFences(text).lines;
}

/**
 * Cells of a markdown table row, with the leading/trailing empties dropped.
 *
 * ESCAPED pipes are honoured; pipes inside code spans are NOT. That asymmetry
 * is GFM's, not ours: a table row is split on unescaped pipes *before* inline
 * parsing, so `| \`a | b\` | c |` really is three cells and renders that way on
 * GitHub. An earlier version blanked pipes inside code spans too, which made
 * this accept a table that renders broken — a false negative, and worse than
 * the false positive it was fixing. To put a pipe in a cell, escape it, even
 * inside a code span.
 */
export function cells(line) {
  const parts = line.replace(/\\\|/g, "\u0000").split("|").map((c) => c.replace(/\u0000/g, "|"));
  if (parts.length && parts[0].trim() === "") parts.shift();
  if (parts.length && parts[parts.length - 1].trim() === "") parts.pop();
  return parts.map((c) => c.trim());
}

/** A `|---|---|` style separator, however it is spaced or aligned. */
export function isTableSeparator(line) {
  return /\|/.test(line) && /-{3,}/.test(line) && /^[\s|:-]+$/.test(line);
}
