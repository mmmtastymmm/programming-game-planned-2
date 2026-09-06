// Markdown helpers shared by the doc checks, so there is one definition of each
// rather than a copy per checker.

// The info string is m[2]. A backtick fence may not carry a backtick in it —
// that is CommonMark's rule, and it is what keeps a lone `` `x` `` span in
// running prose from ever reading as an opener.
const FENCE = /^\s*(`{3,}|~{3,})(.*)$/;

/** Is this line an opener, given what is currently open? The rule, once. */
function fenceEvent(line, open) {
  const m = FENCE.exec(line);
  if (!m) return null;
  const char = m[1][0];
  const len = m[1].length;
  if (!open) {
    if (char === "`" && m[2].includes("`")) return null;
    return { kind: "open", char, len, info: m[2].trim() };
  }
  if (char === open.char && len >= open.len && m[2].trim() === "") return { kind: "close" };
  return null;
}

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
  let open = null; // { char, len, line }
  const lines = text.split("\n").map((line, i) => {
    const ev = fenceEvent(line, open);
    if (ev?.kind === "open") {
      open = { char: ev.char, len: ev.len, line: i + 1, info: ev.info };
      return "";
    }
    if (ev?.kind === "close") {
      open = null;
      return "";
    }
    return open ? "" : line;
  });
  return { lines, unterminated: open };
}

/**
 * The fenced blocks themselves — `{ info, line, body }` each — by the SAME rule
 * scanFences blanks them with.
 *
 * check-mermaid hand-rolled its own `startsWith("```")` scan, which is how two
 * checks came to disagree about what is content: a ```mermaid example quoted
 * inside a ````markdown block is inert to check-structure (there is a mutation
 * asserting exactly that) and was a live diagram to check-mermaid, so writing
 * about a broken diagram failed CI. It also could not see `~~~` fences at all.
 * check-links carries the same note about its own former copy: one definition,
 * or the checks drift apart on the shapes only some of them can see.
 */
export function fencedBlocks(text) {
  const blocks = [];
  let open = null;
  let buf = [];
  text.split("\n").forEach((line, i) => {
    const ev = fenceEvent(line, open);
    if (ev?.kind === "open") {
      open = { char: ev.char, len: ev.len, line: i + 1, info: ev.info };
      buf = [];
      return;
    }
    if (ev?.kind === "close") {
      blocks.push({ info: open.info, line: open.line, body: buf.join("\n") });
      open = null;
      return;
    }
    if (open) buf.push(line);
  });
  return { blocks, unterminated: open };
}

/**
 * Blank out INDENTED code blocks — the other half of "this is code, not content".
 *
 * A fence is not the only way to quote something, and this corpus quotes its own
 * formats constantly. `    **P1 — four-space indent**` renders as a code block on
 * GitHub, which is exactly why a writer reaches for it; but only the fenced
 * spelling was blanked, so that line declared a live register entry that no
 * reader sees as one, and the same text was read as a dangling `P1` citation
 * once the entry rule stopped seeing it. Two ways to quote and one of them
 * checked is the shape check-registers' OPENER comment records for list markers,
 * one level up.
 *
 * The rule is DELIBERATELY narrower than CommonMark's, and narrow in the safe
 * direction: blanking a line the checks should have read makes them go quiet
 * over content, which is this repo's worst failure class, while declining to
 * blank one merely means a quoted example has to use a fence or backticks — two
 * spellings the corpus already uses everywhere. So an indented run counts as
 * code only when all of this holds:
 *
 *   * it is indented four spaces or a tab, CommonMark's threshold;
 *   * a blank line sits immediately above it, so a wrapped paragraph line can
 *     never be mistaken for code; and
 *   * the nearest non-blank line above starts at column 0 and is not a list
 *     marker — inside a list, four spaces is ordinary continuation text, which
 *     is what all nineteen of this corpus's deeply-indented lines actually are.
 *
 * A code block nested inside a list item therefore stays visible to the checks.
 * That is a false NEGATIVE for quoting, and the tolerable direction.
 */
export function blankIndentedCode(lines) {
  const out = lines.slice();
  const isBlank = (l) => l === undefined || l.trim() === "";
  const isIndented = (l) => /^(?: {4}|\t)/.test(l) && l.trim() !== "";
  const isListMarker = (l) => /^(?:[-*+]|\d+[.)])\s/.test(l);

  let inCode = false;
  let lastNonBlank = null; // the last line that was left as content
  for (let i = 0; i < out.length; i++) {
    const line = out[i];
    if (inCode) {
      // A blank line does not end an indented block; a non-blank line that is
      // no longer indented does.
      if (isBlank(line)) continue;
      if (isIndented(line)) {
        out[i] = "";
        continue;
      }
      inCode = false;
      lastNonBlank = line;
      continue;
    }
    if (isBlank(line)) continue;
    const opens =
      isIndented(line) &&
      isBlank(out[i - 1]) &&
      (lastNonBlank === null || (/^\S/.test(lastNonBlank) && !isListMarker(lastNonBlank)));
    if (opens) {
      inCode = true;
      out[i] = "";
      continue;
    }
    lastNonBlank = line;
  }
  return out;
}

/**
 * Every code block, both spellings, blanked — with the unterminated FENCE
 * reported, because nothing downstream can tell a reader about that one.
 *
 * Fences go first: a fenced block's interior is already gone by the time the
 * indent rule looks at it, so a four-space line quoted inside a fence cannot be
 * classified twice.
 */
export function scanCode(text) {
  const { lines, unterminated } = scanFences(text);
  return { lines: blankIndentedCode(lines), unterminated };
}

/** Just the blanked lines. Callers that report the unterminated fence use scanCode. */
export function stripCode(text) {
  return scanCode(text).lines;
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

/**
 * A `|---|---|` style separator, however it is spaced or aligned.
 *
 * GFM asks for ONE or more dashes per cell, not three, so `|--|--|`, `|-|-|` and
 * the common alignment form `|:-:|:-:|` are separators too. Demanding three meant
 * a table written that way was not recognised as a table at all: no header check,
 * no column-count check, and not counted in the "N tables well formed" tick — a
 * writer who reached for `:-:` opted out of the check and was told nothing.
 */
export function isTableSeparator(line) {
  return /\|/.test(line) && /-/.test(line) && /^[\s|:-]+$/.test(line);
}
