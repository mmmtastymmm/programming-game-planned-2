*Closed record — see [../README.md](../README.md). Not spec.*

# Q29 — The opposition roster: which tarot cards ship, what each does, how they escalate, and who picks

## Ruling

*Ruling (2026-09-08):* **the roster is the Fool, and the roster grows one
card at a time under a new question number each.** The first design pass
ships one opposition, `fool`, which builds, hauls and never fights, and it
is enough: a target to learn the language on, and the reference programs a
player reads first. The roster is a **ladder** — each card a directory of
shipped source, each one thing more than the last — and its shape is fixed
here so that adding a card is a question about that card and not about the
roster: the player picks any card for any map; two scripted teams on one
map fight each other as they would a player; and a card's bundles stay
simple enough to read even when that costs them strength.

### The rules

1. **One card ships**: the Fool, in `data/opposition/fool/`, as `docs/04`
   describes it. Its script deploys its bundles at tick 0 and nothing else;
   its bundles build a depot and a second printer from plans and never
   attack, convert or deconstruct.
2. **Each further card is a new question number**, which names the card,
   says what it does that the one below it does not, and ships its
   directory. The names are tarot; which cards, and in what order, is
   decided as each is written and not in advance.
3. **The ladder's order is the order the cards were ruled in.** The menu
   shows them in that order; nothing is locked, since nothing persists
   between matches (Q30). A player picks any card for any map, and any
   number of them.
4. **A script cannot tell teams apart**, so two scripted teams on one map
   fight each other by the same programs they would fight a player with.
   Nothing in a card is written against the player in particular.
5. **A card's bundles are reference programs** (design-invariant DI8): the
   worked examples of the language, kept simple, kept correct as the
   language changes, and readable in the data directory. A card that needed
   a trick the docs do not teach is a card written wrong.
6. **No handicaps by data.** A card is its bundles; a weaker card is a
   simpler program, not the same program with smaller numbers. The
   machine numbers in `data/machines.toml` are the same for every team.

### Consequences

- **The first design pass is complete.** Every numbered doc is written and
  every question issued is answered; T9 closes with this ruling and
  milestone M1 with it.
- **The next card is the first thing to write once the game runs**, since a
  card that attacks can only be tested against a sim that exists. It is a
  question then, not now.
- **`docs/04`'s roster paragraph is one card long** and says so.

## Outcome

- **Docs:** [04-opposition.md](../../04-opposition.md) — the PvE section and
  what the doc leaves open. [00-overview.md](../../00-overview.md) — the
  reserved-docs table. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.
- **Task:** [T9](../../TASKS.md) — closes with this ruling.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| A fixed ladder, named and ruled now — five cards from the Fool to one that does everything | Legible progression and a clear target for the first bundles. Five bundles to write against a sim that does not exist yet, and four names chosen before their programs. |
| **The Fool alone now; each further card its own question, in the order written** *(chosen)* | One bundle, already shipped. The ladder exists as a shape, not a list, and each card is designed when it can be tested. |
| The player picks any card for any map | Kept, as the rule for whatever cards exist. |
| The map names its opposition | Rejected: maps become scenarios, and a new map needs a new script. |
| Data-tuned handicaps on one good bundle | Rejected: a card is its program, and a weaker card is a simpler one. |

### How the answer took its shape

A five-card ladder was proposed with names and roles; the user chose to
start with the Fool. The ruling keeps what the proposal had that did not
depend on the cards — the shape of the ladder, who picks, how scripted
teams treat each other, what a card's bundles are for — and leaves the cards
themselves to be ruled one at a time, each when a sim exists to run it
against. That is the same reasoning the design has used throughout: rule the
mechanism now, the content when it can be tested.
