//! The command log (`docs/06`, The command log) as a data structure: the
//! per-tick sets by sender, the questions the driver asks of it, and the
//! one canonical byte layout every command hashes to. [`wire`] is what
//! goes between peers — the sets, the hashes, the handshake — in that
//! layout; [`exchange`] is the driver's view of the other peers, and
//! [`tcp`] the one transport. Nothing here holds a float, and nothing here
//! decides what a tick does: the exchange delivers sets to the log, and
//! the driver runs the tick when every set is in hand.

pub mod exchange;
pub mod tcp;
pub mod wire;

use sim::{Command, CommandKind, PlanKind, TeamId, TilePos};
use std::collections::{BTreeMap, BTreeSet};

/// Every command every peer applied, by tick, then by sender and
/// submission order. Append-only and complete: `(map, log)` is a replay.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandLog {
    by_tick: BTreeMap<u64, Vec<Command>>,
    /// The peers whose sets are expected every tick, and the ticks for
    /// which each has sent one (an empty set is sent explicitly).
    peers: BTreeSet<TeamId>,
    sets_in: BTreeMap<u64, BTreeSet<TeamId>>,
}

impl CommandLog {
    pub fn new(peers: impl IntoIterator<Item = TeamId>) -> CommandLog {
        CommandLog {
            by_tick: BTreeMap::new(),
            peers: peers.into_iter().collect(),
            sets_in: BTreeMap::new(),
        }
    }

    /// Record a peer's set for a tick: its commands, possibly none.
    pub fn submit_set(&mut self, sender: TeamId, tick: u64, commands: Vec<Command>) {
        let entry = self.by_tick.entry(tick).or_default();
        for c in commands {
            debug_assert_eq!(c.sender, sender);
            debug_assert_eq!(c.tick, tick);
            entry.push(c);
        }
        entry.sort_by(|a, b| a.sender.cmp(&b.sender).then(a.seq.cmp(&b.seq)));
        self.sets_in.entry(tick).or_default().insert(sender);
    }

    /// Append one command, stamped with the peer's next sequence number.
    pub fn push(&mut self, mut c: Command) {
        let seq = self
            .by_tick
            .get(&c.tick)
            .map(|v| v.iter().filter(|x| x.sender == c.sender).count() as u32)
            .unwrap_or(0);
        c.seq = seq;
        let sender = c.sender;
        let tick = c.tick;
        self.submit_set(sender, tick, vec![c]);
    }

    /// A peer stops sending: its sets are no longer awaited.
    pub fn close(&mut self, peer: TeamId) {
        self.peers.remove(&peer);
    }

    /// Whether `peer`'s set for `tick` is in hand.
    pub fn has_set(&self, peer: TeamId, tick: u64) -> bool {
        self.sets_in.get(&tick).is_some_and(|s| s.contains(&peer))
    }

    /// The peers whose sets are awaited every tick.
    pub fn peers(&self) -> impl Iterator<Item = TeamId> + '_ {
        self.peers.iter().copied()
    }

    /// Whether every peer's set for `tick` is in hand.
    pub fn have_all_sets(&self, tick: u64) -> bool {
        let got = self.sets_in.get(&tick);
        self.peers
            .iter()
            .all(|p| got.is_some_and(|s| s.contains(p)))
    }

    /// The peers whose set for `tick` has not arrived.
    pub fn missing_for(&self, tick: u64) -> Vec<TeamId> {
        let got = self.sets_in.get(&tick);
        self.peers
            .iter()
            .filter(|p| !got.is_some_and(|s| s.contains(p)))
            .copied()
            .collect()
    }

    /// The tick's commands, every peer's, ordered by sender then seq.
    pub fn sets_for(&self, tick: u64) -> Vec<Command> {
        self.by_tick.get(&tick).cloned().unwrap_or_default()
    }

    /// The lowest tick above `after` that carries any command (Q32).
    pub fn next_tick_with_a_command(&self, after: u64) -> Option<u64> {
        self.by_tick
            .range(after.saturating_add(1)..)
            .find(|(_, v)| !v.is_empty())
            .map(|(t, _)| *t)
    }

    pub fn commands(&self) -> impl Iterator<Item = &Command> {
        self.by_tick.values().flatten()
    }

    pub fn last_tick(&self) -> Option<u64> {
        self.by_tick.keys().next_back().copied()
    }

    /// The log's canonical bytes, every command length-prefixed, for the
    /// replay's identity.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for c in self.commands() {
            let bytes = encode(c);
            out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            out.extend_from_slice(&bytes);
        }
        out
    }
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u64).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn put_opt_str(out: &mut Vec<u8>, s: Option<&str>) {
    match s {
        None => out.push(0),
        Some(s) => {
            out.push(1);
            put_str(out, s);
        }
    }
}

/// One command's canonical byte layout (`docs/06`, Encoding): spec once
/// written, changed only under a new question number.
pub fn encode(c: &Command) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&c.tick.to_le_bytes());
    out.extend_from_slice(&c.sender.0.to_le_bytes());
    out.extend_from_slice(&c.seq.to_le_bytes());
    match &c.kind {
        CommandKind::Deploy { deployment, bundle } => {
            out.push(1);
            put_str(&mut out, deployment);
            out.extend_from_slice(&(bundle.files.len() as u64).to_le_bytes());
            for (name, text) in &bundle.files {
                put_str(&mut out, name);
                put_str(&mut out, text);
            }
        }
        CommandKind::Mark { at, plan, value } => {
            out.push(2);
            out.extend_from_slice(&at.x.to_le_bytes());
            out.extend_from_slice(&at.y.to_le_bytes());
            out.push(plan_byte(*plan));
            put_opt_str(&mut out, value.as_deref());
        }
        CommandKind::Unmark { at, plan } => {
            out.push(3);
            out.extend_from_slice(&at.x.to_le_bytes());
            out.extend_from_slice(&at.y.to_le_bytes());
            out.push(plan_byte(*plan));
        }
        CommandKind::SetSpeed(s) => {
            out.push(4);
            out.extend_from_slice(&s.to_le_bytes());
        }
        CommandKind::Resign => out.push(5),
    }
    out
}

fn plan_byte(p: PlanKind) -> u8 {
    match p {
        PlanKind::Paint => 0,
        PlanKind::Overlay => 1,
        PlanKind::Building => 2,
    }
}

fn plan_of(b: u8) -> Result<PlanKind, String> {
    match b {
        0 => Ok(PlanKind::Paint),
        1 => Ok(PlanKind::Overlay),
        2 => Ok(PlanKind::Building),
        _ => Err(format!("unknown plan kind byte {b}")),
    }
}

/// A cursor over canonical bytes; every read is bounds-checked, so a
/// truncated or hostile frame is an `Err`, never a panic.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { bytes, at: 0 }
    }

    pub fn done(&self) -> bool {
        self.at == self.bytes.len()
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(n)
            .filter(|e| *e <= self.bytes.len())
            .ok_or_else(|| format!("truncated: wanted {n} bytes at {}", self.at))?;
        let out = &self.bytes[self.at..end];
        self.at = end;
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    pub fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i32(&mut self) -> Result<i32, String> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u64(&mut self) -> Result<u64, String> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    /// A length as `encode` writes one, refused if it cannot fit.
    pub fn length(&mut self) -> Result<usize, String> {
        let n = self.u64()?;
        usize::try_from(n)
            .ok()
            .filter(|n| *n <= self.bytes.len().saturating_sub(self.at))
            .ok_or_else(|| format!("length {n} exceeds the frame"))
    }

    pub fn str(&mut self) -> Result<String, String> {
        let n = self.length()?;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|e| format!("not UTF-8: {e}"))
    }

    pub fn opt_str(&mut self) -> Result<Option<String>, String> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.str()?)),
            b => Err(format!("bad option byte {b}")),
        }
    }
}

/// The inverse of [`encode`]: the command from its canonical bytes, all of
/// them — trailing bytes are a malformed frame.
pub fn decode(bytes: &[u8]) -> Result<Command, String> {
    let mut r = Reader::new(bytes);
    let c = decode_from(&mut r)?;
    if !r.done() {
        return Err("trailing bytes after the command".into());
    }
    Ok(c)
}

/// One command read from the cursor's current position.
pub fn decode_from(r: &mut Reader<'_>) -> Result<Command, String> {
    let tick = r.u64()?;
    let sender = TeamId(r.u32()?);
    let seq = r.u32()?;
    let kind = match r.u8()? {
        1 => {
            let deployment = r.str()?;
            let n = r.length()?;
            let mut files = Vec::with_capacity(n.min(1024));
            for _ in 0..n {
                let name = r.str()?;
                let text = r.str()?;
                files.push((name, text));
            }
            CommandKind::Deploy {
                deployment,
                bundle: sim::Bundle { files },
            }
        }
        2 => {
            let x = r.i32()?;
            let y = r.i32()?;
            let plan = plan_of(r.u8()?)?;
            let value = r.opt_str()?;
            CommandKind::Mark {
                at: TilePos::new(x, y),
                plan,
                value,
            }
        }
        3 => {
            let x = r.i32()?;
            let y = r.i32()?;
            let plan = plan_of(r.u8()?)?;
            CommandKind::Unmark {
                at: TilePos::new(x, y),
                plan,
            }
        }
        4 => CommandKind::SetSpeed(r.u64()?),
        5 => CommandKind::Resign,
        b => return Err(format!("unknown command kind byte {b}")),
    };
    Ok(Command {
        tick,
        sender,
        seq,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::{TilePos, bundle_of};

    fn cmd(tick: u64, sender: u32, kind: CommandKind) -> Command {
        Command::new(tick, TeamId(sender), 0, kind)
    }

    #[test]
    fn sets_order_by_sender_then_seq_and_a_missing_set_stalls() {
        let mut log = CommandLog::new([TeamId(0), TeamId(1)]);
        log.push(cmd(3, 1, CommandKind::Resign));
        log.push(cmd(3, 0, CommandKind::SetSpeed(2)));
        log.push(cmd(3, 0, CommandKind::SetSpeed(5)));
        // A push is a submission: both teams' sets for tick 3 are in.
        assert!(log.have_all_sets(3));
        let seqs: Vec<(u32, u32)> = log
            .sets_for(3)
            .iter()
            .map(|c| (c.sender.0, c.seq))
            .collect();
        assert_eq!(seqs, [(0, 0), (0, 1), (1, 0)]);
        log.submit_set(TeamId(1), 4, vec![]);
        assert!(!log.have_all_sets(4));
        log.submit_set(TeamId(0), 4, vec![]);
        assert!(log.have_all_sets(4));
        assert_eq!(log.next_tick_with_a_command(0), Some(3));
        assert_eq!(log.next_tick_with_a_command(3), None);
    }

    #[test]
    fn every_kind_round_trips_through_its_bytes() {
        let kinds = [
            CommandKind::Deploy {
                deployment: "red".into(),
                bundle: sim::Bundle {
                    files: vec![
                        ("main.py".into(), "x = 1\n".into()),
                        ("util.py".into(), "".into()),
                    ],
                },
            },
            CommandKind::Mark {
                at: TilePos::new(-3, 2),
                plan: PlanKind::Building,
                value: Some("depot".into()),
            },
            CommandKind::Mark {
                at: TilePos::new(0, 0),
                plan: PlanKind::Overlay,
                value: None,
            },
            CommandKind::Unmark {
                at: TilePos::new(5, -5),
                plan: PlanKind::Paint,
            },
            CommandKind::SetSpeed(7),
            CommandKind::Resign,
        ];
        for (i, kind) in kinds.into_iter().enumerate() {
            let c = Command::new(1000 + i as u64, TeamId(i as u32), 3, kind);
            let bytes = encode(&c);
            assert_eq!(decode(&bytes).unwrap(), c);
            // A truncated frame is an error, never a panic, at every cut.
            for cut in 0..bytes.len() {
                assert!(decode(&bytes[..cut]).is_err(), "cut {cut} of {i}");
            }
            let mut extra = bytes.clone();
            extra.push(0);
            assert!(decode(&extra).unwrap_err().contains("trailing"));
        }
        assert!(
            decode(&[0; 17])
                .unwrap_err()
                .contains("unknown command kind")
        );
        // A length claiming more than the frame holds is refused up front.
        let mut huge = encode(&Command::new(1, TeamId(0), 0, CommandKind::Resign));
        huge[16] = 1;
        huge.extend_from_slice(&u64::MAX.to_le_bytes());
        assert!(decode(&huge).unwrap_err().contains("exceeds"));
    }

    #[test]
    fn the_encoding_is_injective_over_the_fields() {
        let a = cmd(
            1,
            0,
            CommandKind::Mark {
                at: TilePos::new(1, 2),
                plan: PlanKind::Paint,
                value: Some("red".into()),
            },
        );
        let b = cmd(
            1,
            0,
            CommandKind::Mark {
                at: TilePos::new(1, 2),
                plan: PlanKind::Paint,
                value: Some("re".into()),
            },
        );
        let c = cmd(
            1,
            0,
            CommandKind::Mark {
                at: TilePos::new(2, 1),
                plan: PlanKind::Paint,
                value: Some("red".into()),
            },
        );
        let d = cmd(
            1,
            0,
            CommandKind::Deploy {
                deployment: "red".into(),
                bundle: bundle_of("x = 1\n"),
            },
        );
        let e = cmd(
            1,
            0,
            CommandKind::Deploy {
                deployment: "red".into(),
                bundle: bundle_of("x = 1 \n"),
            },
        );
        let all = [encode(&a), encode(&b), encode(&c), encode(&d), encode(&e)];
        for (i, x) in all.iter().enumerate() {
            for (j, y) in all.iter().enumerate() {
                assert_eq!(i == j, x == y, "commands {i} and {j}");
            }
        }
    }
}
