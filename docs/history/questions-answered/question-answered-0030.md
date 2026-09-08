*Closed record — see [../README.md](../README.md). Not spec.*

# Q30 — Progression: printers change hands

## Ruling

*Ruling (2026-09-08):* **progression is inside a match, and it is printers
changing hands.** A bot may **convert** an adjacent printer of another team,
after which the printer is its team's: it prints for them, and — by Q24's
rules, which need no change — its new team's cap rises by ten and its next
color unlocks, while the old team's cap falls and its last color locks. A
printer **defends itself**: every tick, it deals one damage to each adjacent
bot of another team, so a lone bot cannot take one, and a conversion is a
fight. There is no progression between matches — no unlocks, no persistent
anything — and none is planned; the ladder of opponents (Q29) is the only
thing that carries from one match to the next, and it carries nothing but
the player's own programs.

This opens the question and answers it in one commit, because the user's
call was clear and small, and because `docs/05` could not be written without
it.

### The rules

Each is hash-affecting; every number is tuning in data.

1. **`convert(id)`** is a bot action (`02`): `id` names an adjacent printer —
   a machine of model `printer`, or a printer-site — of another team. It
   takes `convert_ticks`. When it completes, if the printer still stands and
   the bot is still adjacent, the printer's `team` becomes the bot's; its
   `deployment` becomes the new team's `printer` deployment, so it takes that
   team's printer bundle at its next boundary (Q11) with every variable
   cleared; its store, health, name and any print in progress are kept, and
   a print in progress produces a bot for the new team. Nothing else
   changes.
2. **The cap and the deployments follow the printer** (Q24, unchanged): on
   the tick a printer changes team, its new team has one more printer and
   its old team one fewer, and both teams' caps and unlocked colors are
   recomputed as they would be for a build and a death. Bots the old team
   holds above its new cap are kept; bots on its newly locked color keep
   running until they die.
3. **A printer defends itself.** In the tick's damage step (`06`, step 4),
   every printer deals `defence_damage` — one, per model in data — to every
   bot of another team on a tile adjacent to it, before the death rules run,
   and every tick it stands. A site does not defend. This is the one thing in
   the design that acts without a program, and the reason is that a printer
   is a building: it does not decide, it is simply dangerous to stand next
   to. `defence_damage` is a per-model number, zero for every model but the
   printer, so a defending depot is a data change and not a ruling.
4. **A converting bot is therefore under fire.** With `convert_ticks` at
   ten and bot health at ten, a single full-health bot finishes the
   conversion with nothing to spare and dies on the last tick if the numbers
   are equal; two bots share the risk. That arithmetic is the design's, and
   the numbers in data are set so that a conversion costs a team something.
5. **Converting a team's last printer puts it out** (`04`), on that tick,
   without a death. Its machines are removed in the damage step; the
   converted printer is not among them, since it is no longer theirs.
6. **A team may not convert its own printer**, and a printer of a team that
   is out does not exist to convert. `convert` on anything else is a
   `ValueError` before it begins.
7. **Conversion is loud**, with cause `"convert"`, at the bot's position when
   it begins.
8. **Order**: conversions complete in ascending entity id like every action,
   and a printer converted twice on one tick — two teams' bots finishing
   together — goes to the lower entity id's team.

### What each doc owns

`02` owns the action, the sound cause, the `defence_damage` model number and
the damage-step entry. `04` owns what conversion does to a team's standing.
`05` is this ruling's doc: it says what progression is and is not, and
points at the two rules that make it.

### Consequences

- **A printer is the objective in every sense.** It is what a team needs to
  exist (`04`), what bounds its fleet (Q24), what unlocks its programs
  (Q24), and now what it can take from a rival. Every scripted team and
  every player fights over the same thing.
- **Conversion is bloodless and reversible**, unlike deconstruction (Q28):
  a converted printer can be converted back. That makes a match's
  progression a tug rather than a slide.
- **Between-match progression is a new question number**, if it is ever
  wanted; the `05` doc says so and stays short.

## Outcome

- **Docs:** [05-progression.md](../../05-progression.md) — written from this
  ruling. [02-machines.md](../../02-machines.md) — the action, the cause,
  the defence. [04-opposition.md](../../04-opposition.md) — conversion in
  the ending rule. [00-overview.md](../../00-overview.md) — the
  reserved-docs table. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.
- **Task:** [T9](../../TASKS.md) — `05` leaves its list.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Printers change hands; nothing persists between matches** *(chosen)* | One action, one defence number, and a doc that stays short. Progression is a match's story, told by who holds the printers. |
| A campaign across the tarot ladder with unlocks that persist | Persistent state outside the sim — a save, a profile — and a second design for what an unlock is. Deferred, not rejected. |
| Progression inside a match through research or tiers | A tech tree: a second economy to price and balance, and a reason every match plays the same opening. |
| None at all | The smallest option, and the one the user declined: a match with no way to gain anything from a rival is a match with no arc. |

### How the answer took its shape

The user's rule, taken as given, with one thing added: that the cap and the
color follow the printer *because* Q24 already ties them to printers owned,
so conversion needs no new bookkeeping — it is a `team` field changing and
Q24 doing the rest. The self-defence was the user's too, and the ruling
only had to decide whether it is a program or a rule; a rule, because a
printer is a building, and the design has been consistent that buildings do
not decide. Between-match progression was left out on purpose: the design is
starting small, and a profile is the first thing that would not live in the
sim.
