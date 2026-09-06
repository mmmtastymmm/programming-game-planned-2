// PROVENANCE: ported from the predecessor project (../programming_game_planned).
// The incidents cited below happened THERE. They are kept because they are the
// evidence that justifies the check — not because they happened in this repo.
// Parse every ```mermaid block in the docs with the real mermaid parser.
//
// Why this exists: a broken diagram is invisible in review and in `git diff` —
// it only shows up as a parse error when someone renders the page. The class is
// easy to hit, because mermaid treats characters that read as ordinary prose as
// syntax (`;` terminates a statement, so "pause & save up; more text" silently
// truncates the message and then fails on the remainder).
//
// Reports absolute file:line by mapping mermaid's block-relative line number
// back through the fence position.

import fs from "fs";
import path from "path";
import { JSDOM } from "jsdom";
import { markdownFiles } from "./lib/md-files.mjs";
import { fencedBlocks } from "./lib/markdown.mjs";

const dom = new JSDOM("<!doctype html><body></body>");
global.window = dom.window;
global.document = dom.window.document;

const mermaid = (await import("mermaid")).default;
mermaid.initialize({ startOnLoad: false });

const root = path.resolve(process.argv[2] ?? ".");

// Fences come from lib/markdown.mjs, the same matcher check-structure,
// check-registers and check-links blank content with. This file used to scan for
// a line starting with ``` itself, and the copy could not see what the shared
// one sees: a ```mermaid example quoted inside a ````markdown block was a live
// diagram here and inert everywhere else, so documenting a broken diagram failed
// CI, and `~~~` fences were invisible entirely.

// mermaid reports "Parse error on line N" relative to the block; block line 1
// is the line after the fence, so absolute = fenceLine + N.
function absoluteLine(message, fenceLine) {
  const m = /Parse error on line (\d+)/.exec(message ?? "");
  return m ? fenceLine + Number(m[1]) : fenceLine;
}

let blocks = 0;
const failures = [];

const files = markdownFiles(root).sort();
// Scoring green on zero inputs is how a check that has lost its scope keeps
// printing a tick — the failure the language spike hit with four processes that
// ran nothing.
if (files.length === 0) {
  console.error(`✗ check-mermaid: no markdown under ${root} — the check is checking nothing`);
  process.exit(2);
}

for (const file of files) {
  const { blocks: fenced, unterminated } = fencedBlocks(fs.readFileSync(file, "utf8"));

  for (const block of fenced) {
    // The info string, not a prefix of the line: ```mermaidish is not mermaid.
    if (block.info !== "mermaid") continue;
    blocks++;
    try {
      await mermaid.parse(block.body);
    } catch (err) {
      const msg = String(err?.message ?? err);
      failures.push({
        file: path.relative(process.cwd(), file),
        line: absoluteLine(msg, block.line),
        message: msg.split("\n").slice(0, 4).join("\n    "),
      });
    }
  }

  // Only a mermaid fence left open is this check's business — check-structure
  // reports an unterminated fence of any kind. But an unterminated mermaid block
  // is a diagram that never gets parsed, and saying nothing about it here is how
  // the tick below would come to cover one fewer block than the run before.
  if (unterminated?.info === "mermaid") {
    failures.push({
      file: path.relative(process.cwd(), file),
      line: unterminated.line,
      message: "unterminated ```mermaid block (no closing fence)",
    });
  }
}

for (const f of failures) {
  console.error(`\n✗ ${f.file}:${f.line}\n    ${f.message}`);
}

if (failures.length > 0) {
  console.error(
    `\n${failures.length} of ${blocks} mermaid block(s) failed to parse.`,
  );
  process.exit(1);
}

// Zero FILES is guarded above; zero BLOCKS was not, so the one diagram this
// corpus has could be deleted, renamed, or fenced differently and this step
// would go on printing a tick over nothing — the green-on-nothing result five
// other checks here refuse. If the corpus is ever meant to carry no diagrams,
// the honest change is to drop this step, not to let it pass vacuously.
if (blocks === 0) {
  console.error(
    `✗ check-mermaid: no \`\`\`mermaid blocks under ${root} — the check is checking nothing`,
  );
  process.exit(2);
}

console.log(`✓ ${blocks} mermaid blocks parse clean`);
