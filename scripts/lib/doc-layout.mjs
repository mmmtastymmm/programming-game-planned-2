// The split-doc breadcrumb, in ONE place.
//
// check-doc-layout.mjs enforces it; check-vocabulary.mjs checks that CLAUDE.md,
// which owns the convention in prose, still teaches the same string. They had
// already disagreed: CLAUDE.md gave `*Part of [NN-name](../NN-name.md).*` as the
// formula for every part file, while the checker builds the crumb from the
// IMMEDIATE parent directory — so a nested part written to CLAUDE.md's letter,
// `docs/01-language/runtime/vm.md` opening with `[01-language](../01-language.md)`,
// failed CI with "breadcrumb names the wrong doorway". A rule stated twice and
// checked once is the drift this whole lib directory exists against.

/** The crumb a part file opens with, naming its immediate parent doorway. */
export const breadcrumb = (parent) => `*Part of [${parent}](../${parent}.md).*`;

/**
 * The placeholder CLAUDE.md writes the template with. It is the doorway-name
 * shape, `NN-name`, because the top-level case is the one the prose teaches;
 * the nested case is the same function with a different parent.
 */
export const TEMPLATE_PARENT = "NN-name";
