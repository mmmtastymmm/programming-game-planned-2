//! What goes between peers (`docs/06`, The command log): a set per tick
//! per peer, possibly empty; a hash per completed tick; a goodbye; and the
//! handshake that starts a session. One canonical byte layout per message,
//! commands in [`crate::encode`]'s, framed by a length so a stream can be
//! cut into messages. Spec once written; a change is a new protocol number.

use crate::{Reader, decode_from, encode};
use sim::{Command, TeamId};
use std::io::{Read, Write};

/// Bumped when the layout changes; a session between two numbers is
/// refused at the handshake.
pub const PROTOCOL: u32 = 1;

/// A frame larger than this is refused unread: a bundle is text, and no
/// honest set comes near it.
pub const MAX_FRAME: u32 = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// A joiner's first message: the protocol it speaks and the match it
    /// believes it is joining — the map's hash and the delay — so a peer
    /// on the wrong map is refused before it stalls everyone.
    Hello {
        protocol: u32,
        map_hash: u64,
        delay: u64,
    },
    /// The host's answer: the team the joiner plays.
    Welcome {
        team: TeamId,
    },
    Refuse {
        reason: String,
    },
    /// A peer's command set for a tick — every command it agreed for that
    /// tick, and nothing else; empty is a set too.
    Set {
        sender: TeamId,
        tick: u64,
        commands: Vec<Command>,
    },
    /// A peer's state hash after completing a tick.
    Hash {
        sender: TeamId,
        tick: u64,
        hash: u64,
    },
    /// A peer leaves; its sets are no longer awaited.
    Bye {
        sender: TeamId,
    },
}

impl Message {
    /// The team a relayed message speaks for, if it speaks for one.
    pub fn sender(&self) -> Option<TeamId> {
        match self {
            Message::Set { sender, .. }
            | Message::Hash { sender, .. }
            | Message::Bye { sender } => Some(*sender),
            _ => None,
        }
    }
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u64).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

pub fn encode_message(m: &Message) -> Vec<u8> {
    let mut out = Vec::new();
    match m {
        Message::Hello {
            protocol,
            map_hash,
            delay,
        } => {
            out.push(10);
            out.extend_from_slice(&protocol.to_le_bytes());
            out.extend_from_slice(&map_hash.to_le_bytes());
            out.extend_from_slice(&delay.to_le_bytes());
        }
        Message::Welcome { team } => {
            out.push(11);
            out.extend_from_slice(&team.0.to_le_bytes());
        }
        Message::Refuse { reason } => {
            out.push(12);
            put_str(&mut out, reason);
        }
        Message::Set {
            sender,
            tick,
            commands,
        } => {
            out.push(1);
            out.extend_from_slice(&sender.0.to_le_bytes());
            out.extend_from_slice(&tick.to_le_bytes());
            out.extend_from_slice(&(commands.len() as u64).to_le_bytes());
            for c in commands {
                let bytes = encode(c);
                out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
                out.extend_from_slice(&bytes);
            }
        }
        Message::Hash { sender, tick, hash } => {
            out.push(2);
            out.extend_from_slice(&sender.0.to_le_bytes());
            out.extend_from_slice(&tick.to_le_bytes());
            out.extend_from_slice(&hash.to_le_bytes());
        }
        Message::Bye { sender } => {
            out.push(3);
            out.extend_from_slice(&sender.0.to_le_bytes());
        }
    }
    out
}

/// A message from its bytes. A set's commands must each carry the set's
/// sender and tick: a set is one peer's, for one tick, by definition.
pub fn decode_message(bytes: &[u8]) -> Result<Message, String> {
    let mut r = Reader::new(bytes);
    let m = match r.u8()? {
        10 => Message::Hello {
            protocol: r.u32()?,
            map_hash: r.u64()?,
            delay: r.u64()?,
        },
        11 => Message::Welcome {
            team: TeamId(r.u32()?),
        },
        12 => Message::Refuse { reason: r.str()? },
        1 => {
            let sender = TeamId(r.u32()?);
            let tick = r.u64()?;
            let n = r.length()?;
            let mut commands = Vec::with_capacity(n.min(1024));
            for _ in 0..n {
                let len = r.length()?;
                let mut inner = Reader::new(r.take(len)?);
                let c = decode_from(&mut inner)?;
                if !inner.done() {
                    return Err("trailing bytes after a command".into());
                }
                if c.sender != sender || c.tick != tick {
                    return Err(format!(
                        "a set from team {} for tick {tick} carries a command stamped team {} tick {}",
                        sender.0, c.sender.0, c.tick
                    ));
                }
                commands.push(c);
            }
            Message::Set {
                sender,
                tick,
                commands,
            }
        }
        2 => Message::Hash {
            sender: TeamId(r.u32()?),
            tick: r.u64()?,
            hash: r.u64()?,
        },
        3 => Message::Bye {
            sender: TeamId(r.u32()?),
        },
        b => return Err(format!("unknown message byte {b}")),
    };
    if !r.done() {
        return Err("trailing bytes after the message".into());
    }
    Ok(m)
}

/// Write one message as a frame: a little-endian `u32` length, then the
/// bytes.
pub fn write_frame(w: &mut impl Write, m: &Message) -> std::io::Result<()> {
    let bytes = encode_message(m);
    let len = u32::try_from(bytes.len())
        .ok()
        .filter(|l| *l <= MAX_FRAME)
        .ok_or_else(|| std::io::Error::other("message exceeds the frame cap"))?;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&bytes)?;
    w.flush()
}

/// Read one frame; `Ok(None)` at a clean end of stream before any byte of
/// a frame. A malformed frame is an error, and the stream is unusable
/// after one — the caller closes it.
pub fn read_frame(r: &mut impl Read) -> std::io::Result<Option<Message>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_le_bytes(len);
    if len > MAX_FRAME {
        return Err(std::io::Error::other(format!(
            "frame of {len} bytes exceeds the cap"
        )));
    }
    let mut bytes = vec![0u8; len as usize];
    r.read_exact(&mut bytes)?;
    decode_message(&bytes)
        .map(Some)
        .map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::{CommandKind, bundle_of};

    fn all() -> Vec<Message> {
        vec![
            Message::Hello {
                protocol: PROTOCOL,
                map_hash: 0xdead_beef,
                delay: 3,
            },
            Message::Welcome { team: TeamId(2) },
            Message::Refuse {
                reason: "wrong map".into(),
            },
            Message::Set {
                sender: TeamId(1),
                tick: 9,
                commands: vec![],
            },
            Message::Set {
                sender: TeamId(1),
                tick: 9,
                commands: vec![
                    Command::new(9, TeamId(1), 0, CommandKind::SetSpeed(2)),
                    Command::new(
                        9,
                        TeamId(1),
                        1,
                        CommandKind::Deploy {
                            deployment: "red".into(),
                            bundle: bundle_of("wait(1)\n"),
                        },
                    ),
                ],
            },
            Message::Hash {
                sender: TeamId(0),
                tick: 9,
                hash: 42,
            },
            Message::Bye { sender: TeamId(3) },
        ]
    }

    #[test]
    fn every_message_round_trips_and_every_truncation_is_refused() {
        for m in all() {
            let bytes = encode_message(&m);
            assert_eq!(decode_message(&bytes).unwrap(), m);
            for cut in 0..bytes.len() {
                assert!(decode_message(&bytes[..cut]).is_err(), "{m:?} cut {cut}");
            }
        }
    }

    #[test]
    fn a_set_carrying_another_peers_command_is_refused() {
        let m = Message::Set {
            sender: TeamId(1),
            tick: 9,
            commands: vec![Command::new(9, TeamId(0), 0, CommandKind::Resign)],
        };
        let e = decode_message(&encode_message(&m)).unwrap_err();
        assert!(e.contains("stamped team 0"), "{e}");
        let m = Message::Set {
            sender: TeamId(1),
            tick: 9,
            commands: vec![Command::new(8, TeamId(1), 0, CommandKind::Resign)],
        };
        assert!(decode_message(&encode_message(&m)).is_err());
    }

    #[test]
    fn frames_cut_a_stream_into_messages() {
        let mut stream = Vec::new();
        for m in all() {
            write_frame(&mut stream, &m).unwrap();
        }
        let mut r = stream.as_slice();
        let mut got = Vec::new();
        while let Some(m) = read_frame(&mut r).unwrap() {
            got.push(m);
        }
        assert_eq!(got, all());
        // A frame claiming more than the cap is refused before it is read.
        let mut big = (MAX_FRAME + 1).to_le_bytes().to_vec();
        big.extend_from_slice(&[0; 8]);
        assert!(read_frame(&mut big.as_slice()).is_err());
        // A stream cut inside a frame is an error, not a clean end.
        let cut = &stream[..stream.len() - 1];
        let mut r = cut;
        let mut err = None;
        loop {
            match read_frame(&mut r) {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(e) => {
                    err = Some(e);
                    break;
                }
            }
        }
        assert!(err.is_some());
    }
}
