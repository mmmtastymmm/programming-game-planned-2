//! What a match is started from and what it leaves behind (`docs/07`,
//! Q36): the choices the start screen and the command line share, the
//! driver built from them — off the main thread while a host or joiner
//! waits on its peers — and the replay written when the match is over.
//! Plain Rust, so it is tested headless; [`crate::screens`] is the Bevy
//! half.

use crate::driver::Driver;
use sim::script::Tree;
use sim::{Bundle, CommandKind};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// Play alone against the cards, host on an address, or join one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Play,
    Host(String),
    Join(String),
}

/// Every choice a match is started from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    pub map: PathBuf,
    /// Scripted teams, seated in team order after the player's (`04`).
    pub opposition: Vec<PathBuf>,
    /// The programs directory (Q33); `None` starts with no programs.
    pub programs: Option<PathBuf>,
    pub mode: Mode,
}

/// `data/` beside the workspace, wherever the binary runs from.
pub fn data_root() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    // Resolved, so the paths the screens show read plainly.
    root.canonicalize().unwrap_or(root)
}

impl Setup {
    /// The first map, the Fool, and the starter programs.
    pub fn defaults(root: &Path) -> Setup {
        Setup {
            map: root.join("maps/first.toml"),
            opposition: vec![root.join("opposition/fool")],
            programs: Some(root.join("starter")),
            mode: Mode::Play,
        }
    }

    /// The setup the command line gives, and whether it skips the start
    /// screen: any flag does (Q36), `--play` alone taking every default.
    pub fn from_args(
        args: impl IntoIterator<Item = String>,
        root: &Path,
    ) -> Result<(Setup, bool), String> {
        let mut setup = Setup::defaults(root);
        let mut opposition: Vec<PathBuf> = Vec::new();
        let mut host = None;
        let mut join = None;
        let mut skip = false;
        let mut args = args.into_iter();
        while let Some(a) = args.next() {
            skip = true;
            let mut value = |flag: &str| args.next().ok_or_else(|| format!("{flag} needs a value"));
            match a.as_str() {
                "--play" => {}
                "--host" => host = Some(value("--host ADDR")?),
                "--join" => join = Some(value("--join ADDR")?),
                "--map" => setup.map = PathBuf::from(value("--map FILE")?),
                "--opposition" => opposition.push(PathBuf::from(value("--opposition DIR")?)),
                "--programs" => setup.programs = Some(PathBuf::from(value("--programs DIR")?)),
                "--no-programs" => setup.programs = None,
                other => return Err(format!("unknown argument {other}")),
            }
        }
        setup.mode = match (host, join) {
            (Some(_), Some(_)) => return Err("--host and --join are exclusive".into()),
            (Some(a), None) => Mode::Host(a),
            (None, Some(a)) => Mode::Join(a),
            (None, None) => Mode::Play,
        };
        // Single-player defaults to the Fool; a host seats only what it is
        // given, and a joiner seats nothing.
        setup.opposition = match &setup.mode {
            Mode::Play if opposition.is_empty() => setup.opposition,
            Mode::Join(_) if !opposition.is_empty() => {
                return Err("a joiner seats no opposition; drop --opposition".into());
            }
            _ => opposition,
        };
        Ok((setup, skip))
    }
}

/// The `.toml` files of `data/maps`, or the directories of
/// `data/opposition`, sorted, for the start screen's lists.
pub fn list(dir: &Path, maps: bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            if maps {
                p.extension().is_some_and(|x| x == "toml")
            } else {
                p.is_dir()
            }
        })
        .collect();
    out.sort();
    out
}

/// A path's stem, for naming a map or card on screen and in a file name.
pub fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}

/// A started match: the driver and what the renderer keeps beside it.
pub struct Match {
    pub driver: Driver,
    pub programs: Option<PathBuf>,
    pub tree: Tree,
    pub deployed: BTreeMap<String, Bundle>,
    /// The map file's stem, which names the replay.
    pub map_stem: String,
    /// A programs directory that failed to load, started without.
    pub warning: Option<String>,
}

/// What [`start`] reads before any peer is involved.
struct Loaded {
    map_text: String,
    map_stem: String,
    programs: Option<PathBuf>,
    tree: Tree,
    deployed: BTreeMap<String, Bundle>,
    opening: Vec<CommandKind>,
    warning: Option<String>,
}

fn load(setup: &Setup) -> Result<Loaded, String> {
    let map_text =
        std::fs::read_to_string(&setup.map).map_err(|e| format!("{}: {e}", setup.map.display()))?;
    let mut warning = None;
    let (opening, tree, deployed) = match &setup.programs {
        Some(dir) => match crate::programs::read_all(dir) {
            Ok((tree, bundles)) => (
                crate::programs::deploys_for(&bundles, &Default::default()),
                tree,
                bundles,
            ),
            Err(e) => {
                warning = Some(e);
                (Vec::new(), Default::default(), Default::default())
            }
        },
        None => (Vec::new(), Default::default(), Default::default()),
    };
    Ok(Loaded {
        map_text,
        map_stem: stem(&setup.map),
        programs: setup.programs.clone(),
        tree,
        deployed,
        opening,
        warning,
    })
}

impl Loaded {
    fn into_match(self, driver: Driver) -> Match {
        Match {
            driver,
            programs: self.programs,
            tree: self.tree,
            deployed: self.deployed,
            map_stem: self.map_stem,
            warning: self.warning,
        }
    }
}

/// What the handshake thread reports.
enum Note {
    Progress(String),
    Done(Result<net::tcp::Session, String>),
}

/// A host or joiner waiting on its peers. The handshake holds no sim, so it
/// runs on its own thread; the driver is built on this one once it is done.
pub struct Pending {
    rx: Receiver<Note>,
    loaded: Option<Loaded>,
    opposition: Vec<PathBuf>,
    host: bool,
    /// The handshake's latest word, for the screen.
    pub progress: String,
}

pub enum Started {
    Ready(Box<Match>),
    Waiting(Box<Pending>),
}

/// Read the map and programs and build the driver as the command line
/// always has — at once for play, behind a handshake for host and join.
pub fn start(setup: &Setup) -> Result<Started, String> {
    let loaded = load(setup)?;
    let (tx, rx) = channel();
    let map_text = loaded.map_text.clone();
    let host = match &setup.mode {
        Mode::Play => {
            let dirs: Vec<&Path> = setup.opposition.iter().map(PathBuf::as_path).collect();
            let driver = Driver::new(&loaded.map_text, &dirs, loaded.opening.clone())?;
            return Ok(Started::Ready(Box::new(loaded.into_match(driver))));
        }
        Mode::Host(addr) => {
            let addr = addr.clone();
            let count = setup.opposition.len();
            std::thread::spawn(move || {
                let progress = tx.clone();
                let r = Driver::host_handshake(&map_text, count, &addr, |line| {
                    let _ = progress.send(Note::Progress(line.to_string()));
                });
                let _ = tx.send(Note::Done(r));
            });
            true
        }
        Mode::Join(addr) => {
            let addr = addr.clone();
            std::thread::spawn(move || {
                let _ = tx.send(Note::Progress(format!("joining {addr}")));
                let _ = tx.send(Note::Done(Driver::join_handshake(&map_text, &addr)));
            });
            false
        }
    };
    Ok(Started::Waiting(Box::new(Pending {
        rx,
        loaded: Some(loaded),
        opposition: setup.opposition.clone(),
        host,
        progress: String::new(),
    })))
}

impl Pending {
    /// The match once every peer is in, an error if the handshake failed,
    /// or nothing yet.
    pub fn poll(&mut self) -> Option<Result<Match, String>> {
        loop {
            match self.rx.try_recv() {
                Ok(Note::Progress(line)) => self.progress = line,
                Ok(Note::Done(r)) => return Some(self.finish(r)),
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => {
                    return Some(Err("the handshake ended without a word".into()));
                }
            }
        }
    }

    fn finish(&mut self, session: Result<net::tcp::Session, String>) -> Result<Match, String> {
        let session = session?;
        let loaded = self.loaded.take().ok_or("the match was already started")?;
        let (player, dirs): (sim::TeamId, Vec<&Path>) = if self.host {
            (
                sim::TeamId(0),
                self.opposition.iter().map(PathBuf::as_path).collect(),
            )
        } else {
            (session.me, Vec::new())
        };
        let driver = Driver::with_exchange(
            &loaded.map_text,
            &dirs,
            player,
            loaded.opening.clone(),
            Box::new(session),
        )?;
        Ok(loaded.into_match(driver))
    }
}

/// Where a match's replay goes (Q36): `replays/<map>-<tick>-<hash>.ron`
/// inside the programs directory, or the working directory without one —
/// named by the tick and hash, so two peers that agree write one name.
pub fn replay_path(programs: Option<&Path>, map_stem: &str, tick: u64, hash: u64) -> PathBuf {
    programs
        .unwrap_or(Path::new("."))
        .join("replays")
        .join(format!("{map_stem}-{tick}-{hash:016x}.ron"))
}

/// Write the match so far as its replay; the path it went to.
pub fn save_replay(
    driver: &Driver,
    programs: Option<&Path>,
    map_stem: &str,
) -> Result<PathBuf, String> {
    let path = replay_path(programs, map_stem, driver.tick, driver.state_hash());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&path, driver.replay().to_ron())
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_flag_shows_the_screen_and_any_flag_skips_it() {
        let root = data_root();
        let (setup, skip) = Setup::from_args(args(&[]), &root).unwrap();
        assert!(!skip);
        assert_eq!(setup, Setup::defaults(&root));
        let (setup, skip) = Setup::from_args(args(&["--play"]), &root).unwrap();
        assert!(skip, "--play skips the screen");
        assert_eq!(setup, Setup::defaults(&root), "--play takes every default");
        let (setup, skip) = Setup::from_args(args(&["--no-programs"]), &root).unwrap();
        assert!(skip);
        assert_eq!(setup.programs, None);
    }

    #[test]
    fn the_flags_seat_the_opposition_as_before() {
        let root = data_root();
        let (s, _) = Setup::from_args(args(&["--host", "0.0.0.0:7777"]), &root).unwrap();
        assert_eq!(s.mode, Mode::Host("0.0.0.0:7777".into()));
        assert!(
            s.opposition.is_empty(),
            "a host seats only what it is given"
        );
        let (s, _) = Setup::from_args(args(&["--join", "h:1"]), &root).unwrap();
        assert_eq!(s.mode, Mode::Join("h:1".into()));
        assert!(s.opposition.is_empty());
        let (s, _) =
            Setup::from_args(args(&["--opposition", "a", "--opposition", "b"]), &root).unwrap();
        assert_eq!(s.opposition, vec![PathBuf::from("a"), PathBuf::from("b")]);
        for bad in [
            &["--host", "a", "--join", "b"][..],
            &["--join", "b", "--opposition", "a"],
            &["--map"],
            &["--frobnicate"],
        ] {
            assert!(Setup::from_args(args(bad), &root).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_screen_lists_the_shipped_maps_and_cards() {
        let root = data_root();
        let maps: Vec<String> = list(&root.join("maps"), true)
            .iter()
            .map(|p| stem(p))
            .collect();
        assert!(maps.contains(&"first".to_string()), "{maps:?}");
        let cards: Vec<String> = list(&root.join("opposition"), false)
            .iter()
            .map(|p| stem(p))
            .collect();
        assert!(cards.contains(&"fool".to_string()), "{cards:?}");
    }

    #[test]
    fn play_starts_at_once_and_the_replay_lands_under_the_programs_directory() {
        let root = data_root();
        let mut setup = Setup::defaults(&root);
        setup.programs = None;
        let Started::Ready(mut m) = start(&setup).unwrap() else {
            panic!("play waited on a handshake");
        };
        assert_eq!(m.map_stem, "first");
        m.driver.advance(1.0);
        assert!(m.driver.tick > 0);

        let dir = std::env::temp_dir().join(format!("render-replay-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = save_replay(&m.driver, Some(&dir), &m.map_stem).unwrap();
        let name = format!("first-{}-{:016x}.ron", m.driver.tick, m.driver.state_hash());
        assert_eq!(path, dir.join("replays").join(name));
        let back = replay::Replay::from_ron(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back.run().last().copied(), Some(m.driver.state_hash()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_map_is_an_error_on_the_screen_not_a_crash() {
        let mut setup = Setup::defaults(&data_root());
        setup.map = PathBuf::from("/no/such/map.toml");
        let Err(e) = start(&setup) else {
            panic!("started on a map that does not exist")
        };
        assert!(e.contains("map.toml"), "{e}");
    }

    #[test]
    fn a_join_with_no_host_reports_the_failure_through_poll() {
        let mut setup = Setup::defaults(&data_root());
        setup.opposition.clear();
        // Bind and drop a listener so the port is known closed.
        let addr = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        setup.mode = Mode::Join(addr.to_string());
        let Started::Waiting(mut p) = start(&setup).unwrap() else {
            panic!("a join did not wait on its handshake");
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let r = loop {
            if let Some(r) = p.poll() {
                break r;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "no word from the join"
            );
            std::thread::yield_now();
        };
        let Err(e) = r else { panic!("joined nobody") };
        assert!(e.contains("connect"), "{e}");
    }
}
