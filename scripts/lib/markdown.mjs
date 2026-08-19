// Markdown helpers shared by the doc checks, so there is one definition of each
// rather than a copy per checker.

/**
 * Blank out fenced code blocks, preserving line numbering.
 *
 * This corpus documents its own formats constantly, so a doc quoting
 * "# T99 — illustration" inside a fence must not read as a register entry, and a
 * fenced table must not be checked as a table. Returns an array of lines.
 */
export function stripFences(text) {
  let open = false;
  return text.split("\n").map((line) => {
    if (/^\s*(```|~~~)/.test(line)) {
      open = !open;
      return "";
    }
    return open ? "" : line;
  });
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
