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
 * Inline code spans and escaped pipes are blanked first. A row like
 * `| \`a \\| b\` | bitwise or |` is two columns, not three — and this corpus
 * documents its own syntax, so an operator table is a realistic addition rather
 * than a hypothetical one.
 */
export function cells(line) {
  const safe = line
    .replace(/\\\|/g, "\u0000")
    .replace(/`[^`]*`/g, (m) => m.replace(/\|/g, "\u0000"));
  const parts = safe.split("|").map((c) => c.replace(/\u0000/g, "|"));
  if (parts.length && parts[0].trim() === "") parts.shift();
  if (parts.length && parts[parts.length - 1].trim() === "") parts.pop();
  return parts.map((c) => c.trim());
}

/** A `|---|---|` style separator, however it is spaced or aligned. */
export function isTableSeparator(line) {
  return /\|/.test(line) && /-{3,}/.test(line) && /^[\s|:-]+$/.test(line);
}
