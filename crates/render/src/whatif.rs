//! What-if (`docs/07`, Debugging): *try in the fork* sends a file's
//! deploys into the open fork, and *take it* deploys to the match what
//! that file's last try sent.
//!
//! A try reaches the fork alone, a private copy that never reaches the
//! match (`CLAUDE.md`, rule 1). A take reaches the match the one way
//! anything does: as `Deploy` commands, submitted as the deploy button
//! submits them.

use crate::app::ViewState;
use crate::driver::Driver;
use crate::input::submit;
use bevy_egui::egui;
use sim::{Bundle, CommandKind};

/// What one *try* sent into the fork: each deployment and its bundle.
pub type Tried = Vec<(String, Bundle)>;

/// What the player asked of a file window's what-if buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    Try,
    Take,
}

/// Which of the two a file window shows, as (*try in the fork*, *take
/// it*): both while a fork is open, *try* alone on a replay, which has no
/// match to take it to, and neither in a live PvP match, whose fork takes
/// no what-if.
pub fn shown(d: &Driver) -> (bool, bool) {
    let open = d.debug.session.is_some() && !d.live_pvp();
    (open, open && !d.is_replay())
}

/// The deploys a try of `path` would send: every deployment the file
/// reaches whose bundle is not what the fork's team holds there. One
/// that fails to load refuses them all.
fn deploys(d: &Driver, state: &ViewState, path: &str) -> Result<Tried, String> {
    if let Some(e) = &state.editor.compose_error {
        return Err(e.clone());
    }
    let reaches = state.editor.reaches(path);
    if reaches.is_empty() {
        return Err("this file reaches no deployment".into());
    }
    let mut deploys = Tried::new();
    for deployment in reaches {
        match state.editor.loaded.get(&deployment) {
            Some(Err(e)) => return Err(format!("{deployment}: {e}")),
            Some(Ok(_)) => {
                let running = d.fork_deployed(&deployment);
                if state.editor.deployment_ahead(&deployment, running) {
                    let bundle = state.editor.bundles[&deployment].clone();
                    deploys.push((deployment, bundle));
                }
            }
            None => {}
        }
    }
    if deploys.is_empty() {
        return Err("the fork already runs what this file composes".into());
    }
    Ok(deploys)
}

/// *Try in the fork*: send the file's deploys into the open fork for its
/// next tick, all of them or none, and remember them for *take it*.
/// `true` if they were sent; a refusal is shown and sends nothing.
pub fn try_file(d: &mut Driver, state: &mut ViewState, path: &str) -> bool {
    let sent = deploys(d, state, path).and_then(|deploys| {
        d.fork_try(&deploys)?;
        Ok(deploys)
    });
    let sent = match sent {
        Ok(sent) => sent,
        Err(e) => {
            state.status = format!("try in the fork: refused — {e}");
            return false;
        }
    };
    // No log of the match carries a tried bundle, so the version windows
    // and the breakpoints learn its text here.
    for (_, bundle) in &sent {
        state.debug.catalog.add(bundle);
    }
    let names: Vec<&str> = sent.iter().map(|(d, _)| d.as_str()).collect();
    let next = d.debug.session.as_ref().map(|s| s.fork.position().tick());
    state.status = format!(
        "try in the fork: {} sent for the fork's tick {}",
        names.join(", "),
        next.unwrap_or_default().saturating_add(1)
    );
    state.debug.tried.insert(path.to_string(), sent);
    true
}

/// Whether *take it* is offered for `path`, or why not: only in a match,
/// and only while the tree still composes every bundle the file's last
/// try sent exactly as it was tried, so a take never reverts a file
/// edited since.
pub fn can_take(d: &Driver, state: &ViewState, path: &str) -> Result<(), String> {
    if d.is_replay() {
        return Err("a replay has no match to take it to".into());
    }
    if d.live_pvp() {
        return Err("a live PvP match takes no what-if".into());
    }
    let tried = state.debug.tried.get(path);
    let tried = tried.ok_or("nothing of this file has been tried in the fork")?;
    let held = |(deployment, bundle): &(String, Bundle)| {
        state.editor.bundles.get(deployment) == Some(bundle)
    };
    if !tried.iter().all(held) {
        return Err("the tree has changed since the try: try again".into());
    }
    Ok(())
}

/// *Take it*: deploy to the match the bundles the file's last try sent
/// into the fork, those the team does not already run. `true` if the log
/// took any.
pub fn take_file(d: &mut Driver, state: &mut ViewState, path: &str) -> bool {
    if let Err(e) = can_take(d, state, path) {
        state.status = format!("take it: refused — {e}");
        return false;
    }
    let tried = state.debug.tried.get(path).cloned().unwrap_or_default();
    let player = d.player;
    let team = d.snapshot().teams.iter().find(|t| t.id == player);
    let running = team.map(|t| t.deployments.clone()).unwrap_or_default();
    let ahead = |deployment: &String| {
        let running = running.get(deployment).copied().flatten();
        state.editor.deployment_ahead(deployment, running)
    };
    let tried: Tried = tried.into_iter().filter(|(d, _)| ahead(d)).collect();
    if tried.is_empty() {
        state.status = "take it: the match already runs what was tried".into();
        return false;
    }
    let mut any = false;
    for (deployment, bundle) in tried {
        any |= submit(d, state, CommandKind::Deploy { deployment, bundle });
    }
    any
}

/// A file window's what-if buttons, beside deploy. A button that cannot
/// act is greyed and says why when the pointer rests on it.
pub fn buttons(ui: &mut egui::Ui, d: &Driver, state: &ViewState, path: &str) -> Option<Ask> {
    let (try_shown, take_shown) = shown(d);
    let mut ask = None;
    let mut button = |shown: bool, name: &str, can: Result<(), String>, does: &str, is: Ask| {
        if !shown {
            return;
        }
        let b = ui.add_enabled(can.is_ok(), egui::Button::new(name));
        let b = match &can {
            Ok(()) => b.on_hover_text(does),
            Err(e) => b.on_disabled_hover_text(e.as_str()),
        };
        if b.clicked() {
            ask = Some(is);
        }
    };
    button(
        try_shown,
        "try in the fork",
        d.can_try(),
        "send this file's deploys into the fork, not the match",
        Ask::Try,
    );
    button(
        take_shown,
        "take it",
        can_take(d, state, path),
        "deploy to the match what this file's last try sent into the fork",
        Ask::Take,
    );
    ask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breakpoints::Text;
    use crate::debug::tests::{
        BOT, COUNTER, PRINTER, both, driver, frames, peers, recorded, run_to, session, tuning,
    };
    use crate::debugger::{Dot, click_file, dots, frame_lines, sync};
    use crate::editor::{Editor, deploy_file};
    use sim::script::Tree;
    use sim::{Cause, EntityId, Step};

    /// What the printer's file is edited to: its counter under another
    /// name, so no machine of the match runs it.
    const TRIED: &str = "y = 0\nwhile True:\n    y = y + 2\n    wait(1)\n";
    const PRINTER_FILE: &str = "robots/printer.py";
    const GEO: &str = "lib/geo.py";

    fn state_of(files: &[(&str, &str)]) -> ViewState {
        let mut t = Tree::new();
        for (path, text) in files {
            t.insert((*path).into(), (*text).into());
        }
        ViewState {
            editor: Editor::from_tree(&t),
            ..Default::default()
        }
    }

    /// The tree the tests' match was opened from, the printer's file
    /// edited and not deployed.
    fn edited() -> ViewState {
        state_of(&[(PRINTER_FILE, TRIED), ("robots/bot.py", COUNTER)])
    }

    /// The same tree with a library file, which reaches both robots.
    fn with_library() -> ViewState {
        state_of(&[
            (PRINTER_FILE, COUNTER),
            ("robots/bot.py", COUNTER),
            (GEO, "def one(n):\n    return n\n"),
        ])
    }

    /// The match at tick 40, a fork opened by hand and the windows synced.
    fn forked(st: &mut ViewState) -> Driver {
        let mut d = driver();
        run_to(&mut d, 40);
        d.open_fork().unwrap();
        assert!(sync(&mut d, st));
        d
    }

    fn version(st: &ViewState, deployment: &str) -> u64 {
        *st.editor.loaded[deployment].as_ref().unwrap()
    }

    fn bundle(st: &ViewState, deployment: &str) -> (String, Bundle) {
        (
            deployment.to_string(),
            st.editor.bundles[deployment].clone(),
        )
    }

    /// The version a machine runs in the fork.
    fn runs(d: &Driver, id: EntityId) -> Option<u64> {
        let machines = &session(d).snapshots.cur.machines;
        machines.iter().find(|m| m.record.id == id)?.runs
    }

    /// The version a machine runs in the match.
    fn match_runs(d: &Driver, id: EntityId) -> Option<u64> {
        let machines = &d.snapshot().machines;
        machines.iter().find(|m| m.record.id == id)?.runs
    }

    /// The deploys the player's side has submitted, as (tick, deployment).
    fn deployed(d: &Driver) -> Vec<(u64, String, Bundle)> {
        d.submitted
            .iter()
            .filter_map(|c| match &c.kind {
                CommandKind::Deploy { deployment, bundle } => {
                    Some((c.tick, deployment.clone(), bundle.clone()))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_try_sends_the_files_deploys_into_the_fork_and_never_the_match() {
        let mut st = edited();
        let mut d = forked(&mut st);
        let old = match_runs(&d, PRINTER);
        let new = version(&st, "printer");
        assert_ne!(old, Some(new));
        let (submitted, logged) = (d.submitted.len(), d.log.commands().count());
        assert!(!session(&d).flags.diverged);

        assert!(try_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(
            st.status,
            "try in the fork: printer sent for the fork's tick 41"
        );
        assert!(session(&d).flags.diverged, "the banner says so at once");
        assert_eq!(st.debug.tried[PRINTER_FILE], vec![bundle(&st, "printer")]);
        assert_eq!(d.fork_deployed("printer"), old, "agreed for the next tick");

        // The fork's next tick deploys it, and the printer takes it at
        // its next boundary (Q11); the bot's file was not tried.
        d.fork_step(Step::Tick).unwrap();
        assert_eq!(d.fork_deployed("printer"), Some(new));
        d.fork_step(Step::Tick).unwrap();
        assert_eq!(runs(&d, PRINTER), Some(new));
        assert_eq!(runs(&d, BOT), old);

        // The match took nothing, then or since.
        frames(&mut d, 30);
        assert!(d.tick > 42);
        assert_eq!(match_runs(&d, PRINTER), old);
        assert_eq!(
            (d.submitted.len(), d.log.commands().count()),
            (submitted, logged)
        );
    }

    #[test]
    fn a_try_sends_each_deployment_the_file_reaches_that_the_fork_does_not_run() {
        let mut st = with_library();
        let mut d = forked(&mut st);
        assert!(try_file(&mut d, &mut st, GEO));
        assert_eq!(
            st.status,
            "try in the fork: bot, printer sent for the fork's tick 41"
        );
        let both = vec![bundle(&st, "bot"), bundle(&st, "printer")];
        assert_eq!(st.debug.tried[GEO], both);
        // A robot's file reaches its own deployment alone.
        assert!(try_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(st.debug.tried[PRINTER_FILE], vec![bundle(&st, "printer")]);

        // Once the fork holds them there is nothing to try, and the last
        // try stays what *take it* would deploy. A step into the tick
        // finds them held, though the snapshot drawn there is a tick old.
        d.fork_select(Some(PRINTER)).unwrap();
        let stop = d.fork_step(Step::Line).unwrap();
        assert_eq!(
            (stop.position.tick(), session(&d).snapshots.cur.tick),
            (41, 40)
        );
        assert_eq!(d.fork_deployed("bot"), Some(version(&st, "bot")));
        assert_eq!(d.fork_deployed("depot"), None);
        assert!(!try_file(&mut d, &mut st, GEO));
        assert_eq!(
            st.status,
            "try in the fork: refused — the fork already runs what this file composes"
        );
        assert_eq!(st.debug.tried[GEO], both);

        // The printer's file edited again: its deployment alone is ahead.
        st.editor.files.get_mut(PRINTER_FILE).unwrap().text = TRIED.into();
        st.editor.recompose();
        assert!(try_file(&mut d, &mut st, GEO));
        assert_eq!(st.debug.tried[GEO], vec![bundle(&st, "printer")]);
        assert_eq!(
            st.status,
            "try in the fork: printer sent for the fork's tick 42"
        );
        // That left the bot's bundle of the first try as it was tried,
        // and not the printer's: a take of all of it is not offered.
        st.debug.tried.insert(GEO.into(), both);
        let changed = "the tree has changed since the try: try again";
        assert_eq!(can_take(&d, &st, GEO).unwrap_err(), changed);
    }

    #[test]
    fn a_bundle_that_fails_to_load_is_refused_with_its_error_and_nothing_is_sent() {
        let mut st = with_library();
        let mut d = forked(&mut st);
        st.editor.files.get_mut(PRINTER_FILE).unwrap().text = "x = (\n".into();
        st.editor.recompose();
        // The bot's bundle loads and is ahead; the printer's does not.
        assert!(st.editor.loaded["bot"].is_ok());
        let error = st.editor.loaded["printer"].clone().unwrap_err();
        assert!(!try_file(&mut d, &mut st, GEO));
        assert_eq!(
            st.status,
            format!("try in the fork: refused — printer: {error}")
        );
        assert!(!session(&d).flags.diverged, "not the bot's either");
        assert!(st.debug.tried.is_empty());
        // The bot's own file reaches no bundle that fails.
        assert!(try_file(&mut d, &mut st, "robots/bot.py"));
        assert_eq!(st.debug.tried["robots/bot.py"], vec![bundle(&st, "bot")]);

        // A tree that does not compose sends nothing either.
        let mut st = state_of(&[
            (PRINTER_FILE, TRIED),
            ("lib/a/geo.py", "a = 1\n"),
            ("lib/b/geo.py", "b = 1\n"),
        ]);
        let mut d = forked(&mut st);
        let error = st
            .editor
            .compose_error
            .clone()
            .expect("two files, one name");
        assert!(!try_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(st.status, format!("try in the fork: refused — {error}"));
        assert!(!session(&d).flags.diverged);

        // Nor does a file that reaches no deployment: an empty robot's.
        let mut st = state_of(&[(PRINTER_FILE, TRIED), ("robots/depot.py", "")]);
        let mut d = forked(&mut st);
        assert!(!try_file(&mut d, &mut st, "robots/depot.py"));
        assert_eq!(
            st.status,
            "try in the fork: refused — this file reaches no deployment"
        );
        assert!(!session(&d).flags.diverged && st.debug.tried.is_empty());
    }

    #[test]
    fn a_try_the_fork_refuses_any_deploy_of_sends_none_of_them() {
        let mut st = with_library();
        let mut d = forked(&mut st);
        let old = d.fork_deployed("bot");
        let (_, good) = bundle(&st, "bot");
        assert_ne!(old, Some(version(&st, "bot")));
        // The second names a deployment the team has not, and is refused
        // as a submission is: before the first is sent.
        let deploys = vec![
            ("bot".to_string(), good.clone()),
            ("nope".to_string(), good),
        ];
        let error = d.fork_try(&deploys).unwrap_err();
        let fork = &session(&d).fork;
        assert_eq!(
            error,
            fork.check_what_if("nope", &deploys[1].1).unwrap_err()
        );
        d.fork_step(Step::Tick).unwrap();
        assert_eq!(d.fork_deployed("bot"), old, "the first was not sent");
        assert!(!session(&d).flags.diverged);
    }

    #[test]
    fn a_try_is_offered_only_where_the_fork_takes_one() {
        let mut st = edited();
        let mut d = driver();
        run_to(&mut d, 40);
        assert_eq!(shown(&d), (false, false), "no fork is open");
        assert_eq!(d.can_try().unwrap_err(), "no fork is open");
        assert!(!try_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(st.status, "try in the fork: refused — no fork is open");
        assert!(d.fork_try(&[bundle(&st, "printer")]).is_err());

        d.open_fork().unwrap();
        assert_eq!(shown(&d), (true, true));
        assert_eq!(d.can_try(), Ok(()));
        d.fork_run().unwrap();
        assert_eq!(d.can_try().unwrap_err(), "pause the run to try");
        assert!(!try_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(st.status, "try in the fork: refused — pause the run to try");
        assert!(d.fork_try(&[bundle(&st, "printer")]).is_err());
        assert!(!session(&d).flags.diverged && st.debug.tried.is_empty());
        d.fork_pause().unwrap();
        assert_eq!(d.can_try(), Ok(()));

        // The driver sends all of a try or none of it.
        let nope = ("nope".to_string(), bundle(&st, "printer").1);
        let refused = d.fork_try(&[bundle(&st, "printer"), nope]).unwrap_err();
        assert!(refused.contains("no deployment `nope`"), "{refused}");
        assert!(!session(&d).flags.diverged);
        assert!(try_file(&mut d, &mut st, PRINTER_FILE));
    }

    #[test]
    fn a_live_pvp_match_offers_neither_and_its_fork_takes_no_try() {
        let (mut a, mut b) = peers();
        both(&mut a, &mut b, 40);
        a.open_fork().unwrap();
        let mut st = edited();
        assert!(a.live_pvp());
        assert_eq!(shown(&a), (false, false));
        let refused = a.can_try().unwrap_err();
        assert!(refused.contains("live PvP"), "{refused}");
        assert!(!try_file(&mut a, &mut st, PRINTER_FILE));
        assert_eq!(st.status, format!("try in the fork: refused — {refused}"));
        assert!(!session(&a).flags.diverged && st.debug.tried.is_empty());
        // Nothing was tried, and a take says what the match is.
        st.debug
            .tried
            .insert(PRINTER_FILE.into(), vec![bundle(&st, "printer")]);
        assert!(!take_file(&mut a, &mut st, PRINTER_FILE));
        assert_eq!(
            st.status,
            "take it: refused — a live PvP match takes no what-if"
        );
        assert!(a.submitted.is_empty());
    }

    #[test]
    fn the_tried_files_breakpoints_hit_in_the_fork_and_its_text_is_known() {
        let mut st = edited();
        let mut d = driver();
        run_to(&mut d, 40);
        sync(&mut d, &mut st);
        click_file(&d, &mut st, PRINTER_FILE, 3);
        assert_eq!(st.status, "");
        let text = Text::of_tree(PRINTER_FILE, TRIED);
        let lines = st.debug.breakpoints.files.get(PRINTER_FILE).cloned();
        let dot =
            |d: &Driver, st: &ViewState| dots(d, &st.debug.catalog, lines.as_ref(), None, &text);
        // Nothing runs the edited text: no hit in the match, a grey dot.
        frames(&mut d, 20);
        sync(&mut d, &mut st);
        assert!(d.debug.session.is_none());
        let grey = Dot {
            line: 3,
            live: false,
            scoped: false,
        };
        assert_eq!(dot(&d, &st), vec![grey]);

        d.open_fork().unwrap();
        sync(&mut d, &mut st);
        let at = session(&d).fork.position().tick();
        assert!(try_file(&mut d, &mut st, PRINTER_FILE));
        let stop = d.fork_step(Step::Continue).unwrap();
        assert_eq!((stop.cause, stop.machine), (Cause::Watch, Some(PRINTER)));
        assert!(stop.position.tick() > at);
        // The fork's printer runs a version no log of the match carries,
        // and the windows know its text all the same.
        sync(&mut d, &mut st);
        let live = Dot { live: true, ..grey };
        assert_eq!(dot(&d, &st), vec![live]);
        assert_eq!(frame_lines(&d, &st.debug, &text), vec![3]);
        let versions = st
            .debug
            .catalog
            .versions_of(d.drawn(), sim::TeamId(0), PRINTER_FILE);
        assert!(versions.contains(&text), "{versions:?}");
        let listing = st.debug.catalog.listing(&text);
        assert_eq!(listing.map(|l| l.text.as_str()), Some(TRIED));
        // The match never ran it, and lists no hit.
        assert!(d.debug.hits.is_empty());
        assert_ne!(match_runs(&d, PRINTER), runs(&d, PRINTER));
    }

    #[test]
    fn take_it_deploys_what_was_tried_while_the_tree_still_holds_it() {
        let mut st = edited();
        let mut d = forked(&mut st);
        let nothing = "nothing of this file has been tried in the fork";
        assert_eq!(can_take(&d, &st, PRINTER_FILE).unwrap_err(), nothing);
        assert!(!take_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(st.status, format!("take it: refused — {nothing}"));

        assert!(try_file(&mut d, &mut st, PRINTER_FILE));
        let tried = bundle(&st, "printer");
        assert_eq!(can_take(&d, &st, PRINTER_FILE), Ok(()));
        assert_eq!(can_take(&d, &st, "robots/bot.py").unwrap_err(), nothing);

        // An edit since the try: it would deploy what was never tried.
        st.editor.files.get_mut(PRINTER_FILE).unwrap().text = "wait(1)\n".into();
        st.editor.recompose();
        let changed = "the tree has changed since the try: try again";
        assert_eq!(can_take(&d, &st, PRINTER_FILE).unwrap_err(), changed);
        assert!(!take_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(st.status, format!("take it: refused — {changed}"));
        assert!(deployed(&d).is_empty());
        // The text as it was tried again: the same bundle, so offered.
        st.editor.files.get_mut(PRINTER_FILE).unwrap().text = TRIED.into();
        st.editor.recompose();
        assert_eq!(can_take(&d, &st, PRINTER_FILE), Ok(()));

        // It goes to the match as a deploy does, `delay` ticks on.
        sync(&mut d, &mut st);
        let agreed = d.next_open_tick();
        assert!(take_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(
            deployed(&d),
            vec![(agreed, tried.0.clone(), tried.1.clone())]
        );
        assert_eq!(
            st.status,
            format!("deploy printer: agreed for tick {agreed}")
        );
        frames(&mut d, 30);
        assert!(d.tick > agreed.saturating_add(1));
        assert_eq!(match_runs(&d, PRINTER), Some(version(&st, "printer")));
        // The match runs it now: a second take deploys nothing.
        sync(&mut d, &mut st);
        assert_eq!(can_take(&d, &st, PRINTER_FILE), Ok(()));
        assert!(!take_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(st.status, "take it: the match already runs what was tried");
        assert_eq!(deployed(&d).len(), 1);
    }

    #[test]
    fn take_it_deploys_only_what_the_match_does_not_run_already() {
        let mut st = with_library();
        let mut d = forked(&mut st);
        assert!(try_file(&mut d, &mut st, GEO));
        // The bot's bundle is deployed to the match on its own.
        assert!(deploy_file(&mut d, &mut st, "robots/bot.py"));
        frames(&mut d, 30);
        sync(&mut d, &mut st);
        assert_eq!(deployed(&d).len(), 1);
        let agreed = d.next_open_tick();
        assert!(take_file(&mut d, &mut st, GEO));
        let (_, printer) = bundle(&st, "printer");
        assert_eq!(
            deployed(&d)[1..],
            [(agreed, "printer".to_string(), printer)]
        );

        // Both at once, in the order they were tried.
        let mut st = with_library();
        let mut d = forked(&mut st);
        assert!(try_file(&mut d, &mut st, GEO));
        assert!(take_file(&mut d, &mut st, GEO));
        let names: Vec<String> = deployed(&d).into_iter().map(|d| d.1).collect();
        assert_eq!(names, ["bot", "printer"]);
    }

    #[test]
    fn what_was_tried_goes_with_its_fork() {
        let mut st = edited();
        let mut d = forked(&mut st);
        assert!(try_file(&mut d, &mut st, PRINTER_FILE));
        // The same fork, frame after frame, keeps it.
        frames(&mut d, 3);
        assert!(!sync(&mut d, &mut st));
        assert_eq!(can_take(&d, &st, PRINTER_FILE), Ok(()));
        // Another fork in its place never had the try sent into it.
        d.open_fork().unwrap();
        assert!(sync(&mut d, &mut st));
        assert!(st.debug.tried.is_empty());
        assert!(!session(&d).flags.diverged);
        assert!(try_file(&mut d, &mut st, PRINTER_FILE));
        d.close_fork();
        sync(&mut d, &mut st);
        assert!(st.debug.tried.is_empty());
        assert_eq!(shown(&d), (false, false));
    }

    #[test]
    fn a_replay_offers_a_try_alone() {
        let mut d = Driver::open_replay(recorded(), tuning()).unwrap();
        frames(&mut d, 40);
        d.open_fork().unwrap();
        let mut st = edited();
        assert!(sync(&mut d, &mut st));
        assert_eq!(shown(&d), (true, false));
        assert!(try_file(&mut d, &mut st, PRINTER_FILE));
        assert!(session(&d).flags.diverged);
        d.fork_step(Step::Tick).unwrap();
        d.fork_step(Step::Tick).unwrap();
        assert_eq!(runs(&d, PRINTER), Some(version(&st, "printer")));
        // There is no match to take it to.
        let refused = "a replay has no match to take it to";
        assert_eq!(can_take(&d, &st, PRINTER_FILE).unwrap_err(), refused);
        assert!(!take_file(&mut d, &mut st, PRINTER_FILE));
        assert_eq!(st.status, format!("take it: refused — {refused}"));
    }
}
