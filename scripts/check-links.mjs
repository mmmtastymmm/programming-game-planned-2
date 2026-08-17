#!/usr/bin/env node
// PROVENANCE: ported from the predecessor project (../programming_game_planned).
// The incidents cited below happened THERE. They are kept because they are the
// evidence that justifies the check — not because they happened in this repo.
// Resolve every relative markdown link in a docs tree and report the ones that
// point at a file that does not exist. No deps — plain Node, unlike the mermaid
// check.
//
//   node scripts/check-links.mjs docs
//
// Why this exists: the register files cite each other constantly, and
// docs/history/ is populated by *moving* blocks verbatim out of docs/ — which
// silently turns a correct `](PROBLEMS.md)` into a broken one, because the
// archived copy now sits one directory deeper. Three links rotted that way
// before this check existed.
//
// SCOPE — what this does NOT check, so nobody reads more into a green run:
//   * `#fragment` anchors. The file half of `page.md#section` is checked; the
//     fragment is not. A first version validated them against a hand-rolled
//     GitHub slugger, which was wrong (GitHub maps each space to its own
//     hyphen; that version collapsed runs, so every heading containing an em
//     dash or `&` — most of this corpus — got the wrong slug). It had also
//     never once executed, because the corpus contains zero anchored links.
//     Rather than ship a subtly-wrong validator for a case nobody uses, the
//     run reports how many fragments it skipped; if that number stops being
//     zero, write the slugger properly against GitHub's actual rule
//     (`downcase.gsub(/[^\p{Word}\- ]/u,'').tr(' ','-')`) with a fixture.
// It also validates `[path.md:NN](path.md)` line citations, which are this
// corpus's real rot class: the link half resolves whatever the number says, so a
// deletion higher up the target file moves every citation below it silently.
// Two things are mechanically checkable and both have bitten here — a line past
// end-of-file, and a line that is blank (P32 was a citation that had slid onto
// one). What is NOT checkable is whether line NN still holds the *quoted* text;
// that is why the register's own header says the quoted text is the reliable
// anchor and the number is a convenience.
//
// CITATIONS INSIDE CODE SPANS are checked too, and were the hole that made the
// paragraph above read as a stronger guarantee than it was. PROBLEMS.md cites a
// second row of the file it just linked as a bare `:NN`, and cites some files
// only as `path.md:NN` with no link at all — and both forms live in backticks,
// which this script used to blank out wholesale before looking for anything. So
// twenty-eight citations were invisible to a check whose CI step is labelled
// "line citations in range", and twenty-one of them were broken — mostly
// pre-split monolith line numbers pointing hundreds of lines past the end of a
// nineteen-line part file. The same `:NN` written in running prose ("the same
// entry's rationale at :24") is checked for the same reason: which shape a
// writer reaches for must not decide whether the number is verified.
//
// Resolving a bare `:NN` needs a rule, because the number names no file. The
// rule — now a convention the register follows rather than a guess this script
// makes — is **nearest preceding citation wins**: a bare `:NN` binds to the last
// file named before it, by link or by `path.md:NN` span, and the binding resets
// at every entry header (a line opening with `**` or a heading). Write the file
// out in full whenever that is not the file you mean; a bare number that has to
// reach past an intervening citation is unresolvable to a reader as well.

import { readFileSync, existsSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { markdownFiles } from "./lib/md-files.mjs";

const root = process.argv[2] ?? "docs";
if (!existsSync(root)) {
  console.error(`check-links: no such directory: ${root}`);
  process.exit(2);
}

// Line count + blank-line lookup for a citation target, read once.
const linesCache = new Map();
function linesOf(file) {
  if (!linesCache.has(file)) {
    const l = readFileSync(file, "utf8").split("\n");
    if (l.at(-1) === "") l.pop(); // a trailing newline is not a line
    linesCache.set(file, l);
  }
  return linesCache.get(file);
}

// [text](target) — skip images, and skip anything with a scheme or a leading
// slash, which is somebody else's problem to resolve.
const LINK = /(?<!!)\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;
// An inline code span, and the citation shapes that hide inside one. A span
// naming a file without a number (`01-language/signals-and-logging.md`) still
// rebinds the scope — the register cites that way and then says "now :18".
const CODE = /`[^`\n]*`/g;
const BARE_CITE = /^`:(\d+)`$/;
const PATH_CITE = /^`([\w./-]+\.md)(?::(\d+))?`$/;
// The same bare `:NN` written in running prose rather than in backticks. The
// lookbehind keeps `sim.rs:966`, `world.rs:1860` and `[page.md:24](…)` out: a
// citation that already names its file is somebody else's case, above.
const PROSE_CITE = /(?<![\w/.]):(\d+)(?![\w:])/g;
// A line that ends whatever citation a bare `:NN` could still be reaching back
// to: a heading, or the bolded opener of a register entry or status block.
const SCOPE_BREAK = /^(#{1,6}\s|\*\*)/;

const problems = [];
let checked = 0;
let skippedFragments = 0;
let citations = 0;
let bareCitations = 0;

// A bare `:NN`, in backticks or in prose. With nothing bound, the number is
// unresolvable to a reader too, so that is the finding rather than a skip.
function checkBare(file, i, bound, n, shown) {
  bareCitations++;
  if (!bound) {
    problems.push(`${file}:${i + 1}  bare citation names no file  ${shown}`);
    return;
  }
  checkCitation(file, i, bound.dest, n, `${bound.shown}${shown}`);
}

// One citation, wherever its number came from. `bound` is the file a following
// bare `:NN` resolves against, so the checks below stay in source order.
function checkCitation(file, i, dest, n, shown) {
  citations++;
  const lines = linesOf(dest);
  if (n > lines.length) {
    problems.push(
      `${file}:${i + 1}  citation past EOF  ${shown} → ${relative(".", dest)} (has ${lines.length} lines)`,
    );
  } else if (lines[n - 1].trim() === "") {
    problems.push(`${file}:${i + 1}  citation lands on a blank line  ${shown}`);
  }
}

for (const file of markdownFiles(root).sort()) {
  let inFence = false;
  let bound = null; // { dest, shown } — the last file named in this scope

  readFileSync(file, "utf8")
    .split("\n")
    .forEach((rawLine, i) => {
      if (/^\s*(```|~~~)/.test(rawLine)) {
        inFence = !inFence;
        return;
      }
      if (inFence) return;
      if (SCOPE_BREAK.test(rawLine)) bound = null;

      // Docs quote link syntax constantly — a register entry explaining why
      // `[page.md:24](page.md)` is a weak citation must not be read as a link
      // to page.md. Mask code spans for the link scan, preserving offsets so
      // links and the citations *inside* code spans can be walked in the order
      // they were written: that order is what binds a bare `:NN` to a file.
      const masked = rawLine.replace(CODE, (m) => " ".repeat(m.length));

      const events = [];
      for (const m of masked.matchAll(LINK)) events.push({ at: m.index, link: m });
      for (const m of rawLine.matchAll(CODE)) events.push({ at: m.index, code: m[0] });
      for (const m of masked.matchAll(PROSE_CITE)) events.push({ at: m.index, prose: m });
      events.sort((a, b) => a.at - b.at);

      for (const ev of events) {
        if (ev.link) {
          const [, label, target] = ev.link;
          if (/^[a-z][a-z0-9+.-]*:/i.test(target) || target.startsWith("/")) continue;

          const [pathPart, fragment] = target.split("#");
          if (fragment !== undefined) skippedFragments++;

          // A bare `#section` link has no file half to resolve.
          if (pathPart === "") continue;

          checked++;
          const dest = resolve(dirname(file), pathPart);
          if (!existsSync(dest)) {
            problems.push(`${file}:${i + 1}  missing file  ${target}`);
            continue;
          }
          if (!dest.endsWith(".md")) continue;
          bound = { dest, shown: pathPart };

          // `[path.md:NN](path.md)` — the number lives in the label, so nothing
          // else notices when it drifts off the end or onto a blank line.
          const cite = /:(\d+)\s*$/.exec(label);
          if (cite) checkCitation(file, i, dest, Number(cite[1]), label);
          continue;
        }

        if (ev.code !== undefined) {
          // `path.md:NN` — a citation with no link half at all — and `path.md`,
          // which carries no number but still names the file a later bare `:NN`
          // means. A name that does not resolve is only an error when a number
          // rides on it: prose says `CLAUDE.md` without meaning docs/CLAUDE.md.
          const pathCite = PATH_CITE.exec(ev.code);
          if (pathCite) {
            const dest = resolve(dirname(file), pathCite[1]);
            if (!existsSync(dest)) {
              if (pathCite[2]) problems.push(`${file}:${i + 1}  missing file  ${ev.code}`);
              continue;
            }
            bound = { dest, shown: pathCite[1] };
            if (pathCite[2]) checkCitation(file, i, dest, Number(pathCite[2]), ev.code);
            continue;
          }

          // `:NN` in backticks — nearest preceding citation wins.
          const bare = BARE_CITE.exec(ev.code);
          if (bare) checkBare(file, i, bound, Number(bare[1]), `:${bare[1]}`);
          continue;
        }

        // The same thing written in running prose: "the same entry's rationale
        // at :24". Identical rule, and it has to be, or the shape a writer
        // reaches for decides whether the number is checked.
        checkBare(file, i, bound, Number(ev.prose[1]), `:${ev.prose[1]}`);
      }
    });
}

if (problems.length) {
  console.error(`✗ ${problems.length} broken link(s):\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

const note = skippedFragments ? `, ${skippedFragments} #fragment(s) not checked` : "";
console.log(
  `✓ ${checked} relative links resolve, ${citations} line citations in range ` +
    `(${bareCitations} of them bare \`:NN\`)${note}`,
);
