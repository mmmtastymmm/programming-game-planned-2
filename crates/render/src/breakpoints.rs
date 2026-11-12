//! Breakpoints (`docs/07`, Debugging): the lines the player set in a file
//! window or a version window, the texts the team's machines run, and the
//! watches (`docs/06`) the two make. Plain Rust with no Bevy and no egui
//! type in it, so everything `docs/07` says of a breakpoint — where a click
//! lands, what it is scoped to, how an edit carries it, when it is greyed —
//! is tested headless. Nothing here reaches a peer: a watch changes nothing
//! and is no part of the hash.

use crate::editor::{Editor, robot_of};
use lang::Limits;
use sim::script::bare_name;
use sim::snapshot::{MachineSnapshot, Snapshot};
use sim::{Bundle, EntityId, TeamId, Watch};
use std::collections::{BTreeMap, BTreeSet};

/// One file's text as a machine can run it: what a window shows and what
/// a watch names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Text {
    /// The deployment whose `main.py` this is. `None` for any other file,
    /// which every deployment's bundle carries under the one name.
    pub deployment: Option<String>,
    /// The file as a bundle names it: `main.py`, `nav.py`.
    pub file: String,
    /// The text's content hash (`docs/06`, Content hashes).
    pub content: u64,
}

impl Text {
    /// The tree's file at `path` as the bundles would carry it: a robot
    /// file is its deployment's `main.py`, any other goes by its bare
    /// name, the text verbatim.
    pub fn of_tree(path: &str, text: &str) -> Text {
        let content = lang::content_hash(text.as_bytes());
        match robot_of(path) {
            Some(d) => Text {
                deployment: Some(d.to_string()),
                file: "main.py".into(),
                content,
            },
            None => Text {
                deployment: None,
                file: bare_name(path).to_string(),
                content,
            },
        }
    }

    /// A file of a bundle a machine of `deployment` runs.
    pub fn of_bundle(deployment: Option<&str>, file: &str, content: u64) -> Text {
        Text {
            deployment: deployment.filter(|_| file == "main.py").map(str::to_string),
            file: file.to_string(),
            content,
        }
    }

    /// What a window's title and a button call it: `bot main.py @1a2b3c4d`.
    pub fn label(&self) -> String {
        let short = self.content >> 32;
        match &self.deployment {
            Some(d) => format!("{d} {} @{short:08x}", self.file),
            None => format!("{} @{short:08x}", self.file),
        }
    }
}

/// A version's text and the lines a statement begins on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub text: String,
    pub begins: BTreeSet<u32>,
}

/// The team's own code as its deploys carried it (`docs/07`, Version
/// windows): each bundle version's files, and each file's text by its
/// content hash. Another team's bundles never enter it.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    /// Each bundle version: its files' content hashes by name.
    versions: BTreeMap<u64, BTreeMap<String, u64>>,
    listings: BTreeMap<(String, u64), Listing>,
    /// The bundles already loaded, by each file's name and content hash.
    seen: BTreeSet<Vec<(String, u64)>>,
    /// How many deploys the last reading found.
    read: usize,
}

impl Catalog {
    /// Take in the team's deploys, in any order. A log only grows, so
    /// nothing is read again until there are more of them. A bundle the
    /// sim would not load is no version a machine can run and is skipped.
    pub fn read(&mut self, deploys: &[&Bundle]) {
        if deploys.len() == self.read {
            return;
        }
        self.read = deploys.len();
        for bundle in deploys {
            self.add(bundle);
        }
    }

    /// Take in one bundle of the team's: a deploy of its log, or one a
    /// *try* sent into a fork, which no log of the match carries
    /// (`docs/07`, What-if). A bundle already taken in is not loaded again.
    pub fn add(&mut self, bundle: &Bundle) {
        let key: Vec<(String, u64)> = bundle
            .files
            .iter()
            .map(|(n, s)| (n.clone(), lang::content_hash(s.as_bytes())))
            .collect();
        if !self.seen.insert(key) {
            return;
        }
        let limits = Limits::parse(lang::data::LIMITS_TOML).expect("limits");
        let Ok(loaded) = sim::load_bundle(bundle, &limits) else {
            return;
        };
        for (name, text) in &bundle.files {
            let Some(content) = loaded.program.content.get(name).copied() else {
                continue;
            };
            self.listings
                .entry((name.clone(), content))
                .or_insert_with(|| Listing {
                    text: text.clone(),
                    begins: loaded.program.begin_lines(name),
                });
        }
        self.versions
            .insert(loaded.version, (*loaded.program.content).clone());
    }

    /// The text of a version, if the team deployed it.
    pub fn listing(&self, text: &Text) -> Option<&Listing> {
        self.listings.get(&(text.file.clone(), text.content))
    }

    /// Whether the machine runs exactly this text.
    pub fn runs(&self, m: &MachineSnapshot, text: &Text) -> bool {
        let Some(files) = m.runs.and_then(|v| self.versions.get(&v)) else {
            return false;
        };
        files.get(&text.file) == Some(&text.content)
            && (text.deployment.is_none() || m.record.deployment == text.deployment)
    }

    /// Every file the machine runs, in name order.
    pub fn texts_of(&self, m: &MachineSnapshot) -> Vec<Text> {
        let Some(files) = m.runs.and_then(|v| self.versions.get(&v)) else {
            return Vec::new();
        };
        files
            .iter()
            .map(|(f, c)| Text::of_bundle(m.record.deployment.as_deref(), f, *c))
            .collect()
    }

    /// Each version of the tree file at `path` that a machine of the team
    /// runs: what its file window offers.
    pub fn versions_of(&self, snap: &Snapshot, team: TeamId, path: &str) -> Vec<Text> {
        let own = Text::of_tree(path, "");
        let found: BTreeSet<Text> = snap
            .machines
            .iter()
            .filter(|m| m.record.team == team)
            .flat_map(|m| self.texts_of(m))
            .filter(|t| t.file == own.file && t.deployment == own.deployment)
            .collect();
        found.into_iter().collect()
    }
}

/// The team's machines that run `text`, and that `scope` lets hit: none
/// means the breakpoint is greyed.
pub fn runners(
    catalog: &Catalog,
    snap: &Snapshot,
    team: TeamId,
    text: &Text,
    scope: Option<EntityId>,
) -> Vec<EntityId> {
    snap.machines
        .iter()
        .filter(|m| m.record.team == team && catalog.runs(m, text))
        .map(|m| m.record.id)
        .filter(|id| scope.is_none_or(|s| s == *id))
        .collect()
}

/// What a new breakpoint on `text` is scoped to: the selected machine if
/// it is the team's and runs exactly that text, and otherwise every
/// machine of the team (`None`).
pub fn scope(
    catalog: &Catalog,
    snap: &Snapshot,
    team: TeamId,
    selected: Option<EntityId>,
    text: &Text,
) -> Option<EntityId> {
    let m = snap
        .machines
        .iter()
        .find(|m| Some(m.record.id) == selected)?;
    (m.record.team == team && catalog.runs(m, text)).then_some(m.record.id)
}

/// One text's breakpoints: each line, and the one machine it is scoped
/// to, or every machine of the team.
pub type Lines = BTreeMap<u32, Option<EntityId>>;

/// Where a breakpoint asked for on `line` sits: the line itself if a
/// statement begins there, else the next one that does; nowhere past the
/// last.
pub fn settle(line: u32, begins: &BTreeSet<u32>) -> Option<u32> {
    begins.range(line..).next().copied()
}

/// A click in the gutter at `line`: set a breakpoint where it settles, or
/// clear what is already there — every breakpoint that settles on the
/// same statement, since an edit can leave one on a line where none
/// begins any more, and one stranded past the last statement if the click
/// is on it. With no `begins` — the text does not load, so nobody knows
/// yet where its statements are — the line is taken as given. Returns
/// the line changed.
pub fn toggle(
    lines: &mut Lines,
    line: u32,
    begins: Option<&BTreeSet<u32>>,
    scope: Option<EntityId>,
) -> Option<u32> {
    let Some(begins) = begins else {
        if lines.remove(&line).is_none() {
            lines.insert(line, scope);
        }
        return Some(line);
    };
    let Some(at) = settle(line, begins) else {
        return lines.remove(&line).map(|_| line);
    };
    let there: Vec<u32> = lines
        .keys()
        .copied()
        .filter(|l| settle(*l, begins) == Some(at))
        .collect();
    if there.is_empty() {
        lines.insert(at, scope);
    }
    for l in there {
        lines.remove(&l);
    }
    Some(at)
}

/// Carry breakpoints through an edit (`docs/07`): a line above the change
/// stays, a line below moves with the text, and a line inside it stays on
/// its number while the change still reaches that far, else goes to the
/// change's last line — or, the change being a deletion, to the line that
/// closed up behind it. Two that land together are one.
pub fn carry(lines: &Lines, old: &str, new: &str) -> Lines {
    let was: Vec<&str> = old.split('\n').collect();
    let now: Vec<&str> = new.split('\n').collect();
    let shortest = was.len().min(now.len());
    let head = was.iter().zip(&now).take_while(|(a, b)| a == b).count();
    let tail = was
        .iter()
        .rev()
        .zip(now.iter().rev())
        .take(shortest - head)
        .take_while(|(a, b)| a == b)
        .count();
    let last = now.len() - 1;
    let mut out = Lines::new();
    for (line, scope) in lines {
        let at = (*line as usize).saturating_sub(1);
        let to = if at < head {
            at
        } else if at >= was.len() - tail {
            (at + now.len()).saturating_sub(was.len())
        } else if now.len() - tail > head {
            at.min(now.len() - tail - 1)
        } else {
            head
        };
        out.entry(to.min(last) as u32 + 1).or_insert(*scope);
    }
    out
}

/// Every breakpoint the interface holds (`docs/07`, Breakpoints).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Breakpoints {
    /// A file window's, by tree path: carried through edits and kept with
    /// the tree.
    pub files: BTreeMap<String, Lines>,
    /// A version window's, kept while the match or replay is open.
    pub versions: BTreeMap<Text, Lines>,
}

impl Breakpoints {
    /// The watches the breakpoints make for `team`, in a fixed order. A
    /// file window's are on the tree's text as it stands, each settled on
    /// a line where a statement begins if the text loads; one past the
    /// last statement watches nothing.
    pub fn watches(&self, team: TeamId, editor: &Editor) -> Vec<Watch> {
        let mut out = Vec::new();
        let mut push = |text: &Text, line: u32, machine: Option<EntityId>| {
            out.push(Watch {
                team,
                file: text.file.clone(),
                content: text.content,
                line,
                deployment: text.deployment.clone(),
                machine,
            });
        };
        for (path, lines) in &self.files {
            let Some(doc) = editor.files.get(path) else {
                continue;
            };
            let text = Text::of_tree(path, &doc.text);
            for (line, machine) in lines {
                let at = match editor.begins.get(path) {
                    Some(b) => settle(*line, b),
                    None => Some(*line),
                };
                if let Some(at) = at {
                    push(&text, at, *machine);
                }
            }
        }
        for (text, lines) in &self.versions {
            for (line, machine) in lines {
                push(text, *line, *machine);
            }
        }
        let key = |w: &Watch| {
            (
                w.file.clone(),
                w.content,
                w.line,
                w.deployment.clone(),
                w.machine,
            )
        };
        out.sort_by_key(key);
        out.dedup();
        out
    }

    /// Follow the tree: a renamed file keeps its breakpoints under its new
    /// path, a deleted one's go with it, and a text with none left is
    /// forgotten.
    pub fn follow(&mut self, moved: &[(String, String)], editor: &Editor) {
        for (from, to) in moved {
            if let Some(lines) = self.files.remove(from) {
                self.files.insert(to.clone(), lines);
            }
        }
        self.files
            .retain(|p, l| !l.is_empty() && editor.files.contains_key(p));
        self.versions.retain(|_, l| !l.is_empty());
    }

    /// The file windows' lines, as the layout keeps them beside the tree.
    /// What a breakpoint is scoped to is a machine of one match and is
    /// not kept.
    pub fn kept(&self) -> BTreeMap<String, Vec<u32>> {
        self.files
            .iter()
            .filter(|(_, l)| !l.is_empty())
            .map(|(p, l)| (p.clone(), l.keys().copied().collect()))
            .collect()
    }

    /// Breakpoints as the layout kept them, each on every machine of the
    /// team.
    pub fn from_kept(kept: &BTreeMap<String, Vec<u32>>) -> Breakpoints {
        Breakpoints {
            files: kept
                .iter()
                .map(|(p, l)| (p.clone(), l.iter().map(|n| (*n, None)).collect()))
                .collect(),
            versions: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::tests::{
        BOT, COUNTER, PRINTER, both, driver, peers, recorded, run_to, tuning,
    };
    use crate::driver::Driver;
    use sim::script::Tree;
    use sim::{CommandKind, bundle_of};

    const TEAM: TeamId = TeamId(0);
    /// The rival's printer.
    const RIVAL: EntityId = EntityId(2);
    /// A bot's text after an edit: no machine runs it until it is deployed.
    const NEXT: &str = "y = 0\nwhile True:\n    wait(1)\n";

    fn hash(text: &str) -> u64 {
        lang::content_hash(text.as_bytes())
    }

    fn bot_text() -> Text {
        Text::of_tree("robots/bot.py", COUNTER)
    }

    fn catalog(d: &Driver) -> Catalog {
        let mut c = Catalog::default();
        c.read(&d.deploys(TEAM));
        c
    }

    fn lines(of: &[(u32, Option<EntityId>)]) -> Lines {
        of.iter().copied().collect()
    }

    fn set(of: &[u32]) -> BTreeSet<u32> {
        of.iter().copied().collect()
    }

    #[test]
    fn a_text_is_named_as_the_bundles_carry_it() {
        let robot = bot_text();
        assert_eq!(robot.deployment.as_deref(), Some("bot"));
        assert_eq!(
            (robot.file.as_str(), robot.content),
            ("main.py", hash(COUNTER))
        );
        let shared = Text::of_tree("lib/geo.py", "a = 1\n");
        assert_eq!(shared.deployment, None);
        assert_eq!(
            (shared.file.as_str(), shared.content),
            ("geo.py", hash("a = 1\n"))
        );
        // The text is hashed verbatim: a space more is another text.
        assert_ne!(Text::of_tree("lib/geo.py", "a = 1 \n"), shared);

        assert_eq!(
            Text::of_bundle(Some("bot"), "main.py", hash(COUNTER)),
            robot
        );
        // Every deployment's bundle carries a shared file under one name.
        assert_eq!(
            Text::of_bundle(Some("bot"), "geo.py", shared.content),
            shared
        );
        assert_eq!(Text::of_bundle(None, "main.py", 7).deployment, None);

        let at = Text {
            content: 0x1a2b_3c4d_0000_0001,
            ..robot
        };
        assert_eq!(at.label(), "bot main.py @1a2b3c4d");
        let at = Text {
            content: 0x0000_0abc_ffff_ffff,
            ..shared
        };
        assert_eq!(at.label(), "geo.py @00000abc");
    }

    #[test]
    fn the_catalog_holds_each_version_the_team_deployed_and_none_that_does_not_load() {
        let counter = bundle_of(COUNTER);
        let broken = bundle_of("x = (\n");
        let next = bundle_of(NEXT);
        let mut c = Catalog::default();
        c.read(&[&counter, &broken]);
        let listing = c.listing(&bot_text()).expect("the counter");
        assert_eq!(listing.text, COUNTER);
        assert_eq!(listing.begins, set(&[1, 2, 3, 4]));
        // Any deployment's `main.py` of that text is the one listing.
        assert_eq!(
            c.listing(&Text::of_tree("robots/printer.py", COUNTER)),
            Some(listing)
        );
        assert_eq!(c.listing(&Text::of_tree("robots/bot.py", "x = (\n")), None);
        assert_eq!(c.listing(&Text::of_tree("robots/bot.py", NEXT)), None);

        // A log only grows: the same count is not read again, more is.
        c.read(&[&counter, &next]);
        assert_eq!(c.listing(&Text::of_tree("robots/bot.py", NEXT)), None);
        c.read(&[&counter, &broken, &next]);
        let listing = c
            .listing(&Text::of_tree("robots/bot.py", NEXT))
            .expect("read");
        assert_eq!(listing.begins, set(&[1, 2, 3]));

        // A bundle's other files are listed under their own names.
        let two = Bundle {
            files: vec![
                ("main.py".into(), "import geo\nwait(geo.one())\n".into()),
                ("geo.py".into(), "\ndef one():\n    return 1\n".into()),
            ],
        };
        c.read(&[&counter, &broken, &next, &two]);
        let geo = Text::of_tree("lib/geo.py", "\ndef one():\n    return 1\n");
        assert_eq!(c.listing(&geo).expect("geo").begins, set(&[2, 3]));
    }

    #[test]
    fn a_machine_runs_a_text_by_its_bundles_version_and_its_deployment() {
        let mut d = driver();
        run_to(&mut d, 40);
        let c = catalog(&d);
        let snap = d.drawn();
        let of = |id: EntityId| snap.machines.iter().find(|m| m.record.id == id).unwrap();
        let printer_text = Text::of_tree("robots/printer.py", COUNTER);
        assert!(c.runs(of(BOT), &bot_text()));
        assert!(!c.runs(of(PRINTER), &bot_text()), "another deployment's");
        assert!(c.runs(of(PRINTER), &printer_text));
        // A text of no one deployment is run by every machine carrying it.
        let any = Text {
            deployment: None,
            ..bot_text()
        };
        assert!(c.runs(of(BOT), &any) && c.runs(of(PRINTER), &any));
        assert!(!c.runs(of(BOT), &Text::of_tree("robots/bot.py", NEXT)));
        let other_file = Text {
            file: "nav.py".into(),
            ..any.clone()
        };
        assert!(!c.runs(of(BOT), &other_file));
        // The rival's bundles never entered the catalog.
        assert!(of(RIVAL).runs.is_some());
        assert!(!c.runs(of(RIVAL), &any));
        assert_eq!(c.texts_of(of(RIVAL)), Vec::<Text>::new());

        assert_eq!(c.texts_of(of(BOT)), vec![bot_text()]);
        assert_eq!(c.texts_of(of(PRINTER)), vec![printer_text.clone()]);
        assert_eq!(c.versions_of(snap, TEAM, "robots/bot.py"), vec![bot_text()]);
        assert_eq!(
            c.versions_of(snap, TEAM, "robots/printer.py"),
            vec![printer_text]
        );
        assert_eq!(
            c.versions_of(snap, TeamId(1), "robots/bot.py"),
            Vec::<Text>::new()
        );
        assert_eq!(c.versions_of(snap, TEAM, "lib/geo.py"), Vec::<Text>::new());

        assert_eq!(runners(&c, snap, TEAM, &bot_text(), None), vec![BOT]);
        assert_eq!(runners(&c, snap, TEAM, &any, None), vec![PRINTER, BOT]);
        assert_eq!(runners(&c, snap, TEAM, &any, Some(BOT)), vec![BOT]);
        assert_eq!(
            runners(&c, snap, TEAM, &bot_text(), Some(PRINTER)),
            Vec::new()
        );
        assert_eq!(runners(&c, snap, TeamId(1), &any, None), Vec::new());
        let edited = Text::of_tree("robots/bot.py", NEXT);
        assert_eq!(runners(&c, snap, TEAM, &edited, None), Vec::new(), "greyed");
    }

    #[test]
    fn a_file_window_offers_each_version_a_machine_still_runs() {
        let mut d = driver();
        run_to(&mut d, 40);
        let was = d.drawn().machines.iter().find(|m| m.record.id == BOT);
        let was = was.unwrap().runs;
        d.submit(CommandKind::Deploy {
            deployment: "bot".into(),
            bundle: bundle_of(NEXT),
        })
        .unwrap();
        run_to(&mut d, 70);
        let c = catalog(&d);
        let next = Text::of_tree("robots/bot.py", NEXT);
        // Every bot took the deploy at its next boundary: the text they
        // ran is one no machine runs, and its breakpoints are greyed.
        let snap = d.drawn();
        let bots = runners(&c, snap, TEAM, &next, None);
        assert!(bots.len() >= 2 && bots.contains(&BOT), "{bots:?}");
        assert_eq!(
            c.versions_of(snap, TEAM, "robots/bot.py"),
            vec![next.clone()]
        );
        assert_eq!(runners(&c, snap, TEAM, &bot_text(), None), Vec::new());
        assert!(c.listing(&bot_text()).is_some(), "its window can stay open");
        let printer = Text::of_tree("robots/printer.py", COUNTER);
        assert_eq!(runners(&c, snap, TEAM, &printer, None), vec![PRINTER]);

        // A bot still on the old version: the window offers both, once
        // each however many machines run one.
        let mut snap = snap.clone();
        let old = snap.machines.iter_mut().find(|m| m.record.id == BOT);
        old.unwrap().runs = was;
        let mut both = vec![bot_text(), next.clone()];
        both.sort();
        assert_eq!(c.versions_of(&snap, TEAM, "robots/bot.py"), both);
        assert_eq!(runners(&c, &snap, TEAM, &bot_text(), None), vec![BOT]);
        assert_eq!(runners(&c, &snap, TEAM, &next, None).len(), bots.len() - 1);

        // A replay's catalog is read from what it recorded.
        let r = Driver::open_replay(recorded(), tuning()).unwrap();
        let c = catalog(&r);
        assert!(c.listing(&bot_text()).is_some());
        assert!(c.listing(&next).is_none());
    }

    #[test]
    fn a_breakpoint_is_scoped_to_the_selected_machine_only_if_it_runs_that_text() {
        let mut d = driver();
        run_to(&mut d, 40);
        let c = catalog(&d);
        let snap = d.drawn();
        let scoped = |selected, text: &Text| scope(&c, snap, TEAM, selected, text);
        assert_eq!(scoped(Some(BOT), &bot_text()), Some(BOT));
        assert_eq!(
            scoped(Some(PRINTER), &bot_text()),
            None,
            "it runs another text"
        );
        assert_eq!(scoped(None, &bot_text()), None);
        assert_eq!(scoped(Some(EntityId(999)), &bot_text()), None);
        let edited = Text::of_tree("robots/bot.py", NEXT);
        assert_eq!(scoped(Some(BOT), &edited), None, "not exactly that text");
        // Never another team's machine, whatever it runs.
        let any = Text {
            deployment: None,
            ..bot_text()
        };
        assert_eq!(scoped(Some(RIVAL), &any), None);
        assert_eq!(scope(&c, snap, TeamId(1), Some(BOT), &bot_text()), None);
    }

    #[test]
    fn another_teams_machines_are_never_a_texts_though_they_run_the_same_bytes() {
        // Both peers deployed the counter, so the rival's printer runs a
        // version this team's catalog holds.
        let (mut a, mut b) = peers();
        both(&mut a, &mut b, 10);
        let c = catalog(&a);
        let snap = a.drawn();
        let of = |id: EntityId| snap.machines.iter().find(|m| m.record.id == id).unwrap();
        let printer = Text::of_tree("robots/printer.py", COUNTER);
        assert_eq!(of(RIVAL).record.team, TeamId(1));
        assert!(c.runs(of(RIVAL), &printer), "the same bytes");
        assert_eq!(runners(&c, snap, TEAM, &printer, None), vec![PRINTER]);
        assert_eq!(runners(&c, snap, TEAM, &printer, Some(RIVAL)), Vec::new());
        assert_eq!(scope(&c, snap, TEAM, Some(RIVAL), &printer), None);
        assert_eq!(
            scope(&c, snap, TEAM, Some(PRINTER), &printer),
            Some(PRINTER)
        );
        assert_eq!(
            c.versions_of(snap, TEAM, "robots/printer.py"),
            vec![printer.clone()]
        );
        // With the team's own printer on another version, the window
        // offers nothing: the rival's is not the team's to open.
        let mut snap = snap.clone();
        let mine = snap.machines.iter_mut().find(|m| m.record.id == PRINTER);
        mine.unwrap().runs = Some(1);
        assert_eq!(
            c.versions_of(&snap, TEAM, "robots/printer.py"),
            Vec::<Text>::new()
        );
        assert_eq!(runners(&c, &snap, TEAM, &printer, None), Vec::new());
    }

    #[test]
    fn a_breakpoint_settles_on_the_next_line_where_a_statement_begins() {
        let begins = set(&[1, 4, 5]);
        assert_eq!(settle(1, &begins), Some(1));
        assert_eq!(settle(2, &begins), Some(4));
        assert_eq!(settle(4, &begins), Some(4));
        assert_eq!(settle(5, &begins), Some(5));
        assert_eq!(settle(6, &begins), None, "past the last statement");
        assert_eq!(settle(1, &BTreeSet::new()), None);
    }

    #[test]
    fn a_gutter_click_sets_a_breakpoint_or_clears_the_one_on_that_statement() {
        let begins = set(&[1, 4, 5]);
        let mut l = Lines::new();
        assert_eq!(toggle(&mut l, 2, Some(&begins), None), Some(4));
        assert_eq!(l, lines(&[(4, None)]));
        // A click on a line that settles there clears it.
        assert_eq!(toggle(&mut l, 3, Some(&begins), Some(BOT)), Some(4));
        assert_eq!(l, Lines::new());
        assert_eq!(toggle(&mut l, 4, Some(&begins), Some(BOT)), Some(4));
        assert_eq!(l, lines(&[(4, Some(BOT))]));
        assert_eq!(toggle(&mut l, 5, Some(&begins), None), Some(5));
        assert_eq!(l, lines(&[(4, Some(BOT)), (5, None)]));

        // An edit can leave breakpoints where no statement begins: one
        // click clears every one that settles on the statement, and
        // leaves the rest.
        let mut l = lines(&[(1, None), (2, None), (3, Some(BOT)), (7, None)]);
        assert_eq!(toggle(&mut l, 4, Some(&begins), None), Some(4));
        assert_eq!(l, lines(&[(1, None), (7, None)]));
        // Past the last statement nothing is set, and a stranded one is
        // cleared by a click on it.
        assert_eq!(toggle(&mut l, 6, Some(&begins), None), None);
        assert_eq!(l, lines(&[(1, None), (7, None)]));
        assert_eq!(toggle(&mut l, 7, Some(&begins), None), Some(7));
        assert_eq!(l, lines(&[(1, None)]));

        // A text that does not load: the line as given.
        let mut l = Lines::new();
        assert_eq!(toggle(&mut l, 9, None, Some(BOT)), Some(9));
        assert_eq!(l, lines(&[(9, Some(BOT))]));
        assert_eq!(toggle(&mut l, 8, None, None), Some(8));
        assert_eq!(toggle(&mut l, 9, None, None), Some(9));
        assert_eq!(l, lines(&[(8, None)]));
    }

    #[test]
    fn an_edit_carries_the_breakpoints_with_the_text() {
        let all = |n: &[u32]| lines(&n.iter().map(|n| (*n, None)).collect::<Vec<_>>());
        let at = |l: &Lines| l.keys().copied().collect::<Vec<u32>>();
        // A line typed above moves those below it.
        let moved = carry(&all(&[1, 2, 3]), "a\nb\nc", "a\nX\nb\nc");
        assert_eq!(at(&moved), vec![1, 3, 4]);
        // A line typed below moves nothing.
        let moved = carry(&all(&[1, 2]), "a\nb\n", "a\nb\nc\n");
        assert_eq!(at(&moved), vec![1, 2]);
        // A change inside a line leaves it where it is.
        let moved = carry(&all(&[1, 2, 3]), "a\nb\nc", "a\nB!\nc");
        assert_eq!(at(&moved), vec![1, 2, 3]);
        // Deleted lines' breakpoints go to the line that closed up behind
        // them, and two that land together are one: the first's scope.
        let l = lines(&[(2, Some(BOT)), (3, None), (4, Some(PRINTER))]);
        let moved = carry(&l, "a\nb\nc\nd", "a\nd");
        assert_eq!(moved, lines(&[(2, Some(BOT))]));
        // Lines replaced by fewer: inside the change while it reaches that
        // far, else on its last line; below it, with the text.
        let moved = carry(&all(&[2, 3, 4, 5]), "a\nb\nc\nd\ne", "a\nX\ne");
        assert_eq!(at(&moved), vec![2, 3]);
        let moved = carry(&all(&[2, 3]), "a\nb\nc\nd", "a\nX\nY\nZ\nW\nd");
        assert_eq!(at(&moved), vec![2, 3]);
        // The last lines deleted: on the last line left.
        let moved = carry(&all(&[1, 3]), "a\nb\nc", "a");
        assert_eq!(at(&moved), vec![1]);
        let moved = carry(&all(&[2]), "a\nb", "");
        assert_eq!(at(&moved), vec![1]);
        // No change, no move; and a line past the text's end stays in it.
        let same = lines(&[(2, Some(BOT)), (3, None)]);
        assert_eq!(carry(&same, "a\nb\nc", "a\nb\nc"), same);
        assert_eq!(at(&carry(&all(&[9]), "a\nb", "a\nb")), vec![2]);
        // A repeated line is told apart from the top, then from the end.
        let moved = carry(&all(&[1, 2, 3]), "a\na\na", "a\na\na\na");
        assert_eq!(at(&moved), vec![1, 2, 3]);
        // The second of two alike deleted: what the top did not match is
        // all the end may claim.
        assert_eq!(at(&carry(&all(&[3]), "a\nb\nb\nc", "a\nb\nc")), vec![3]);
        // Deleted with lines left below: on the first of those.
        let moved = carry(&all(&[3]), "a\nx\ny\nc\nd", "a\nc\nd");
        assert_eq!(at(&moved), vec![2]);
        // Replaced by fewer with lines left below: on the change's last.
        let moved = carry(&all(&[4]), "a\nx\ny\nz\nc\nd", "a\nq\nc\nd");
        assert_eq!(at(&moved), vec![2]);
    }

    fn editor() -> Editor {
        let mut t = Tree::new();
        t.insert(
            "robots/bot.py".into(),
            "import geo\n\n# a note\nx = geo.one()\nwait(x)\n".into(),
        );
        t.insert("lib/geo.py".into(), "def one():\n    return 1\n".into());
        Editor::from_tree(&t)
    }

    #[test]
    fn the_breakpoints_make_the_teams_watches_in_a_fixed_order() {
        let mut e = editor();
        let bot = Text::of_tree("robots/bot.py", &e.files["robots/bot.py"].text);
        let geo = Text::of_tree("lib/geo.py", &e.files["lib/geo.py"].text);
        let version = Text {
            deployment: None,
            file: "main.py".into(),
            content: 0,
        };
        let mut b = Breakpoints::default();
        b.files.insert(
            "robots/bot.py".into(),
            lines(&[(2, None), (3, None), (4, Some(BOT)), (9, None)]),
        );
        b.files.insert("lib/geo.py".into(), lines(&[(1, None)]));
        b.files.insert("lib/gone.py".into(), lines(&[(1, None)]));
        b.versions
            .insert(version.clone(), lines(&[(5, Some(PRINTER)), (2, None)]));
        let watch = |text: &Text, line, machine| Watch {
            team: TEAM,
            file: text.file.clone(),
            content: text.content,
            line,
            deployment: text.deployment.clone(),
            machine,
        };
        // By file, text and line: the two that settle on line 4 for the
        // team are one watch, the one past the last statement is none,
        // and a file no longer in the tree has none.
        assert_eq!(
            b.watches(TEAM, &e),
            vec![
                watch(&geo, 1, None),
                watch(&version, 2, None),
                watch(&version, 5, Some(PRINTER)),
                watch(&bot, 4, None),
                watch(&bot, 4, Some(BOT)),
            ]
        );
        assert!(b.watches(TeamId(1), &e).iter().all(|w| w.team == TeamId(1)));

        // A text that does not load is watched on the lines as given, by
        // its hash as it stands.
        e.files.get_mut("robots/bot.py").unwrap().text = "x = (\n".into();
        e.recompose();
        let broken = Text::of_tree("robots/bot.py", "x = (\n");
        let made = b.watches(TEAM, &e);
        let on: Vec<u32> = made
            .iter()
            .filter(|w| w.content == broken.content)
            .map(|w| w.line)
            .collect();
        assert_eq!(on, vec![2, 3, 4, 9]);
        assert!(made.iter().all(|w| w.content != bot.content));
    }

    #[test]
    fn the_breakpoints_follow_the_tree_through_a_rename_and_a_delete() {
        let mut e = editor();
        let mut b = Breakpoints::default();
        b.files
            .insert("lib/geo.py".into(), lines(&[(1, Some(BOT))]));
        b.files.insert("robots/bot.py".into(), Lines::new());
        b.files
            .insert("interrupts/on_fault.py".into(), lines(&[(2, None)]));
        let kept = Text::of_bundle(None, "nav.py", 1);
        b.versions.insert(kept.clone(), lines(&[(3, None)]));
        b.versions
            .insert(Text::of_bundle(None, "nav.py", 2), Lines::new());

        e.rename("lib/geo.py", "lib/nav.py").unwrap();
        let moved = std::mem::take(&mut e.moved);
        b.follow(&moved, &e);
        let paths: Vec<&str> = b.files.keys().map(String::as_str).collect();
        assert_eq!(paths, vec!["interrupts/on_fault.py", "lib/nav.py"]);
        assert_eq!(b.files["lib/nav.py"], lines(&[(1, Some(BOT))]));
        assert_eq!(b.versions.keys().collect::<Vec<_>>(), vec![&kept]);

        e.delete("lib/nav.py").unwrap();
        b.follow(&[], &e);
        let paths: Vec<&str> = b.files.keys().map(String::as_str).collect();
        assert_eq!(paths, vec!["interrupts/on_fault.py"]);
    }

    #[test]
    fn the_file_windows_lines_are_kept_with_the_tree_without_their_scope() {
        let mut b = Breakpoints::default();
        b.files
            .insert("a.py".into(), lines(&[(3, Some(BOT)), (1, None)]));
        b.files.insert("b.py".into(), Lines::new());
        b.versions.insert(bot_text(), lines(&[(2, None)]));
        let kept = b.kept();
        let expected: BTreeMap<String, Vec<u32>> = [("a.py".to_string(), vec![1, 3])].into();
        assert_eq!(kept, expected);
        let back = Breakpoints::from_kept(&kept);
        assert_eq!(back.files["a.py"], lines(&[(1, None), (3, None)]));
        assert!(back.versions.is_empty());
        assert_eq!(back.kept(), kept);
    }
}
