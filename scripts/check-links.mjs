#!/usr/bin/env node
// PROVENANCE: ported from the predecessor project (../programming_game_planned).
// The incidents cited below happened THERE. They are kept because they are the
// evidence that justifies the check — not because they happened in this repo.
// Resolve every relative markdown link in a docs tree and report the ones that
// point at a file that does not exist. No deps — plain Node, unlike the mermaid
// check.
//
//   node scripts/check-links.mjs .           # the REPO root, not docs/
//
// Why this exists: the register files cite each other constantly, and
// docs/history/ is populated by *moving* blocks verbatim out of docs/ — which
// silently turns a correct `](PROBLEMS.md)` into a broken one, because the
// archived copy now sits one directory deeper. Three links rotted that way
// before this check existed.
//
// LINK SHAPES. Only inline `[text](target)` is understood. CommonMark's
// reference form (`[text][ref]` plus a `[ref]: target` definition) and an
// angle-bracket destination (`[text](<a b.md>)`) are REFUSED rather than
// skipped: skipping them meant a broken reference link left the "N relative
// links resolve" count unmoved, which is the same mislabelled-count failure the
// citation paragraphs below are about. This corpus writes inline links
// exclusively, so refusing costs nothing and keeps the count honest.
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
// It also validates `[path:NN](path)` line citations, which are this
// corpus's real rot class: the link half resolves whatever the number says, so a
// deletion higher up the target file moves every citation below it silently.
// The target does NOT have to be markdown — a citation into `crates/sim/src/rng.rs`
// rots exactly as quietly, and CLAUDE.md links that file. Only a directory is
// exempt, having no lines to land on.
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

import { readFileSync, existsSync, statSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { markdownFiles } from "./lib/md-files.mjs";
import { stripCode } from "./lib/markdown.mjs";

const root = process.argv[2] ?? ".";
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
// ANY extension, not just `.md`. Demanding markdown meant a citation into the
// source tree — `crates/sim/src/rng.rs:9999`, the one non-markdown path this
// corpus actually links — was neither resolved nor counted, while the CI step
// went on saying "N line citations in range". A span that names no real file and
// carries no number is still skipped in silence, because prose says
// `clippy.pedantic` and `1.5` without meaning a path.
const PATH_CITE = /^`([\w./-]+\.[\w]+)(?::(\d+))?`$/;
// The same bare `:NN` written in running prose rather than in backticks. The
// lookbehind keeps a citation that already names its file — `sim.rs:966` — out
// of this scan; PATH_CITE above owns that shape.
const PROSE_CITE = /(?<![\w/.]):(\d+)(?![\w:])/g;
// Link syntax again, for masking the label out of the prose scan. `[the note at
// :99](page.md)` is the LABEL's citation and is checked there; the lookbehind
// above does not exclude it, because the character before its colon is a space,
// so the same number was checked twice and counted twice — inflating the very
// count the CI step advertises.
const LINK_SPAN = /(?<!!)\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;
// Everything between `](` and the matching `)`, however malformed — LINK only
// matches destinations it can resolve, so a shape it rejects has to be found
// some other way before it can be reported rather than skipped.
const DEST = /(?<!!)\[[^\]]*\]\(([^)]*)\)/g;
// A line that ends whatever citation a bare `:NN` could still be reaching back
// to: a heading, or the bolded opener of a register entry or status block.
const SCOPE_BREAK = /^(#{1,6}\s|\*\*)/;

// A check that scores green on zero inputs is this repo's recurring failure —
// four checkers were guarded against it and this one, whose CI step runs first,
// was missed.
if (markdownFiles(root).length === 0) {
  console.error(`✗ check-links: no markdown under ${root} — the check is checking nothing`);
  process.exit(2);
}

const problems = [];
let checked = 0;
let skippedFragments = 0;
let citations = 0;
let bareCitations = 0;

// A bare `:NN`, in backticks or in prose. With nothing bound, the number is
// unresolvable to a reader too, so that is the finding rather than a skip.
function checkBare(file, i, bound, raw, shown) {
  bareCitations++;
  if (!bound) {
    problems.push(`${file}:${i + 1}  bare citation names no file  ${shown}`);
    return;
  }
  checkCitation(file, i, bound.dest, raw, `${bound.shown}${shown}`);
}

// One citation, wherever its number came from. `bound` is the file a following
// bare `:NN` resolves against, so the checks below stay in source order.
//
// `raw` is the digits AS WRITTEN, never a Number. Every citation shape captures
// `(\d+)`, which happily matches "0" and "012" — and the range guard used to
// test only the upper bound, so `:0` indexed lines[-1] and threw a TypeError
// that aborted the whole scan, leaving every file sorted after it unchecked. A
// leading zero was worse than a crash: `Number("012")` is 12, so the citation
// silently pointed somewhere the writer never wrote.
function checkCitation(file, i, dest, raw, shown) {
  citations++;
  if (!/^[1-9][0-9]*$/.test(raw)) {
    problems.push(
      `${file}:${i + 1}  malformed line citation ${shown} — line numbers start at 1 ` +
        `and carry no leading zero`,
    );
    return;
  }
  const n = Number(raw);
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
  let bound = null; // { dest, shown } — the last file named in this scope

  // Fenced lines arrive already blanked, by the SAME matcher the other checks
  // use. This file used to carry its own parity toggle, so a fence shape that
  // fooled one checker fooled a different set of lines here — and an
  // unterminated fence silently ended the scan. lib/markdown.mjs owns the rule
  // now, and check-structure reports the unterminated case.
  stripCode(readFileSync(file, "utf8"))
    .forEach((rawLine, i) => {
      if (SCOPE_BREAK.test(rawLine)) bound = null;

      // Docs quote link syntax constantly — a register entry explaining why
      // `[page.md:24](page.md)` is a weak citation must not be read as a link
      // to page.md. Mask code spans for the link scan, preserving offsets so
      // links and the citations *inside* code spans can be walked in the order
      // they were written: that order is what binds a bare `:NN` to a file.
      const masked = rawLine.replace(CODE, (m) => " ".repeat(m.length));

      // A reference definition or a reference use — neither resolvable by the
      // inline matcher below, and both silently invisible before this.
      if (/^\s{0,3}\[[^\]]+\]:\s*\S/.test(masked)) {
        problems.push(
          `${file}:${i + 1}  reference-style link definition — this corpus writes inline ` +
            `links, and a reference definition is not resolved here`,
        );
      } else if (/\]\[/.test(masked)) {
        problems.push(
          `${file}:${i + 1}  reference-style link — write it inline as [text](target), which ` +
            `is the only shape this check resolves`,
        );
      }
      if (/\]\(</.test(masked)) {
        problems.push(
          `${file}:${i + 1}  angle-bracket link destination — write [text](target) without ` +
            `the angle brackets, which is the only shape this check resolves`,
        );
      }

      // A destination with a raw space in it — `[the doc](a b.md)`. LINK cannot
      // match it (`[^)\s]+` stops at the space) and neither can the angle-bracket
      // guard above, so it was SKIPPED: the "N relative links resolve" count sat
      // unmoved while CommonMark rendered the whole thing as literal text, which
      // is a link that resolves nowhere. Same class as the reference-style shape
      // refused a few lines up, and refused the same way rather than skipped, for
      // the same reason — a count that does not move is a count that lies.
      //
      // A title is the one legal reason for a space after the destination, so it
      // is allowed through in all three of CommonMark's quotings.
      for (const m of masked.matchAll(DEST)) {
        const inner = m[1];
        if (inner.startsWith("<")) continue; // angle-bracket form, refused above
        if (!/^\S*(?:\s+(?:"[^"]*"|'[^']*'|\([^)]*\)))?\s*$/.test(inner)) {
          problems.push(
            `${file}:${i + 1}  link destination contains an unescaped space  (${inner}) — ` +
              `this renders as literal text, not a link; percent-encode the space as %20`,
          );
        }
      }

      const events = [];
      for (const m of masked.matchAll(LINK)) events.push({ at: m.index, link: m });
      for (const m of rawLine.matchAll(CODE)) events.push({ at: m.index, code: m[0] });
      const proseOnly = masked.replace(LINK_SPAN, (m) => " ".repeat(m.length));
      for (const m of proseOnly.matchAll(PROSE_CITE)) events.push({ at: m.index, prose: m });
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
            // `bound` is cleared on the way out of EVERY branch that fails to
            // produce a binding, here and in the code-span scan below. Naming a
            // file ends whatever the previous name bound, whether or not the new
            // name resolves — leaving the old binding in place means a following
            // bare `:NN` is silently checked against a file the writer did not
            // name, and reported under that file's path.
            bound = null;
            problems.push(`${file}:${i + 1}  missing file  ${target}`);
            continue;
          }
          // A DIRECTORY has no lines, so it can neither carry a citation nor be
          // what a later bare `:NN` means. Everything else can: the `.md` filter
          // that used to sit here skipped both jobs for every source-file link,
          // and CLAUDE.md links crates/sim/src/rng.rs, so that is not a
          // hypothetical target. `[rng.rs:9999](../crates/sim/src/rng.rs)` was
          // neither verified nor counted — the `continue` landed above the label
          // scan — while the CI step reported "0 line citations in range", the
          // mislabelled count this file's header is about. Worse, `bound` stayed
          // pointed at whatever `.md` came last, so a following bare `:NN` was
          // silently checked against a file the writer did not name.
          if (!statSync(dest).isFile()) {
            bound = null;
            continue;
          }
          bound = { dest, shown: pathPart };

          // `[path.md:NN](path.md)` — the number lives in the label, so nothing
          // else notices when it drifts off the end or onto a blank line.
          //
          // EVERY `:NN` in the label, not just one at the end. The end-anchored
          // version left `[PROBLEMS.md:9999 for that](PROBLEMS.md)` unchecked AND
          // uncounted — invisible to the prose scan too, whose lookbehind
          // excludes a colon preceded by a word character — while the CI step
          // went on reporting "N line citations in range". Which shape a writer
          // reaches for must not decide whether the number is verified.
          //
          // `(?<!\d)` keeps clock times and ratios out: the colon in "12:30" or
          // "2:1" is preceded by a digit, and neither is a line citation.
          for (const cite of label.matchAll(/(?<!\d):(\d+)(?![\w:])/g)) {
            checkCitation(file, i, dest, cite[1], label);
          }
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
            if (!existsSync(dest) || !statSync(dest).isFile()) {
              // The silent half of this branch is the dangerous one. A code span
              // naming a file that does not exist — a typo, `PROBLMS.md` — is not
              // reported unless a number rides on it, and it used to leave the
              // PREVIOUS binding standing: `` `PROBLMS.md`, at `:5` `` then
              // resolved against whatever .md was named before it, reported
              // "INBOX.md:5", and went green whenever that line happened to
              // exist. The citation into the misspelled file was never checked at
              // all. Same remedy the directory branch above already applies.
              bound = null;
              if (pathCite[2]) problems.push(`${file}:${i + 1}  missing file  ${ev.code}`);
              continue;
            }
            bound = { dest, shown: pathCite[1] };
            if (pathCite[2]) checkCitation(file, i, dest, pathCite[2], ev.code);
            continue;
          }

          // `:NN` in backticks — nearest preceding citation wins.
          const bare = BARE_CITE.exec(ev.code);
          if (bare) {
            checkBare(file, i, bound, bare[1], `:${bare[1]}`);
            continue;
          }
          // A span that carries a `:NN` but matches NEITHER shape — `see :24`,
          // `PROBLEMS.md at :24` — used to fall through here unchecked and
          // uncounted, while the CI step still reported "N line citations in
          // range". That is the same "invisible to a check whose label claims
          // otherwise" incident this rewrite exists to fix, one shape further
          // in. Refuse it rather than skip it: the writer gets told which two
          // forms are verified.
          if (/(?<![\w/.]):\d+(?![\w:])/.test(ev.code)) {
            problems.push(
              `${file}:${i + 1}  unverifiable citation ${ev.code} — a line citation in a code ` +
                `span must be exactly \`:NN\` or \`path.md:NN\`, or CI cannot resolve it`,
            );
          }
          continue;
        }

        // The same thing written in running prose: "the same entry's rationale
        // at :24". Identical rule, and it has to be, or the shape a writer
        // reaches for decides whether the number is checked.
        checkBare(file, i, bound, ev.prose[1], `:${ev.prose[1]}`);
      }
    });
}

if (problems.length) {
  console.error(`✗ ${problems.length} broken link(s):\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

// The fragment count prints even at zero: the SCOPE note above tells a reader to
// watch it ("if that number stops being zero, write the slugger properly"), and
// suppressing it made a broken counter and an unanchored corpus produce the same
// success line.
console.log(
  `✓ ${checked} relative links resolve, ${citations} line citations in range ` +
    `(${bareCitations} of them bare \`:NN\`), ${skippedFragments} #fragment(s) not checked`,
);

// And the other direction of the same worry, said out loud rather than left to a
// zero nobody reads. Citation resolution is the largest thing in this file and
// the live corpus contains none, so the run exercises none of it: an edit that
// broke the scanner outright — a swallowed exception, a regex that stops matching
// — would print a line byte-identical to today's. check-mermaid hard-exits on
// zero blocks for this exact reason; that is not available here, because zero is
// the corpus's honest answer. So the count is annotated instead, and the day it
// stops being zero this note stops printing.
if (citations === 0) {
  console.log(
    "  note: no line citations in the corpus, so nothing exercised the resolver this run — " +
      "check-checks.mjs seeds them; a green tick here says nothing about that half of the file",
  );
}
