//! The one transport: TCP, a star. The host listens, assigns each joiner
//! a team, and relays every set, hash and goodbye it receives to the other
//! joiners, so each peer speaks to one socket and every peer sees every
//! set. A reader thread per connection turns frames into [`Event`]s on a
//! channel the driver polls; the driver thread never blocks on a socket.
//!
//! The handshake is synchronous and happens before a match starts:
//! [`host`] returns once every remote team has connected, and [`join`]
//! once the host has said which team the joiner plays — or refused it.

use crate::exchange::{Event, Exchange};
use crate::wire::{Message, PROTOCOL, read_frame, write_frame};
use sim::TeamId;
use std::io::{BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::thread;

/// What both sides of the handshake compare: the same map and the same
/// delay, or no match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchIdentity {
    pub map_hash: u64,
    pub delay: u64,
}

/// A writer half, shared between the driver's `send` and the relay.
type Writer = Arc<Mutex<TcpStream>>;

struct Link {
    team: TeamId,
    writer: Writer,
}

/// A live session: the host's, carrying every joiner, or a joiner's,
/// carrying the host.
pub struct Session {
    /// This side's team.
    pub me: TeamId,
    /// The remote peers, in team order; on a joiner, every team that is
    /// not its own, since the host relays them all.
    peers: Vec<TeamId>,
    links: Vec<Link>,
    rx: Receiver<Event>,
    lost: Vec<TeamId>,
}

fn write_to(w: &Writer, m: &Message) {
    if let Ok(mut s) = w.lock() {
        // A failed write is noticed by the reader of the same socket,
        // which reports the loss; nothing to do here.
        let _ = write_frame(&mut *s, m);
    }
}

/// Read frames from `stream` until it ends or errs, forwarding each to
/// `tx` and, on the host, to every other link. Every exit reports the loss
/// of `team` — the sets it carried are not coming.
fn reader(
    stream: TcpStream,
    team: TeamId,
    tx: Sender<Event>,
    relay: Arc<Vec<Link>>,
    lost_means: Vec<TeamId>,
) {
    let mut r = BufReader::new(stream);
    // A clean end and an error both mean the same thing here: the peer is
    // gone, and the sets it carried are not coming.
    while let Ok(Some(m)) = read_frame(&mut r) {
        // The host relays what a joiner says to the other joiners; a
        // joiner has nobody to relay to.
        if let Some(sender) = m.sender() {
            for l in relay.iter() {
                if l.team != team && sender != l.team {
                    write_to(&l.writer, &m);
                }
            }
        }
        if tx.send(Event::Message(m)).is_err() {
            return;
        }
    }
    for t in lost_means {
        let _ = tx.send(Event::Lost(t));
    }
    // The other joiners hear the goodbye the host cannot say otherwise.
    for l in relay.iter() {
        if l.team != team {
            write_to(&l.writer, &Message::Bye { sender: team });
        }
    }
}

/// Listen on `addr` and wait for one joiner per team in `remote`, in
/// order; each is checked against `identity` and told its team. Returns
/// the session and the address actually bound (for an ephemeral port).
pub fn host(
    addr: impl ToSocketAddrs,
    me: TeamId,
    remote: &[TeamId],
    identity: MatchIdentity,
    mut progress: impl FnMut(&str),
) -> Result<(Session, SocketAddr), String> {
    let listener = TcpListener::bind(addr).map_err(|e| format!("cannot listen: {e}"))?;
    let bound = listener
        .local_addr()
        .map_err(|e| format!("no local address: {e}"))?;
    let mut streams: Vec<(TeamId, TcpStream)> = Vec::new();
    for &team in remote {
        progress(&format!(
            "waiting on {bound} for team {}'s peer ({} of {})",
            team.0,
            streams.len().saturating_add(1),
            remote.len()
        ));
        loop {
            let (stream, from) = listener
                .accept()
                .map_err(|e| format!("accept failed: {e}"))?;
            match handshake_host(&stream, identity) {
                Ok(()) => {
                    let mut w = &stream;
                    write_frame(&mut w, &Message::Welcome { team })
                        .map_err(|e| format!("cannot welcome {from}: {e}"))?;
                    progress(&format!("team {} is {from}", team.0));
                    streams.push((team, stream));
                    break;
                }
                Err(reason) => {
                    progress(&format!("refused {from}: {reason}"));
                    let mut w = &stream;
                    let _ = write_frame(&mut w, &Message::Refuse { reason });
                }
            }
        }
    }
    let links: Vec<Link> = streams
        .iter()
        .map(|(team, s)| {
            Ok(Link {
                team: *team,
                writer: Arc::new(Mutex::new(
                    s.try_clone()
                        .map_err(|e| format!("cannot clone socket: {e}"))?,
                )),
            })
        })
        .collect::<Result<_, String>>()?;
    let relay = Arc::new(links);
    let (tx, rx) = channel();
    for (team, stream) in streams {
        let _ = stream.set_nodelay(true);
        let tx = tx.clone();
        let relay = relay.clone();
        thread::spawn(move || reader(stream, team, tx, relay, vec![team]));
    }
    let links = relay
        .iter()
        .map(|l| Link {
            team: l.team,
            writer: l.writer.clone(),
        })
        .collect();
    Ok((
        Session {
            me,
            peers: remote.to_vec(),
            links,
            rx,
            lost: Vec::new(),
        },
        bound,
    ))
}

fn handshake_host(stream: &TcpStream, identity: MatchIdentity) -> Result<(), String> {
    let mut r = BufReader::new(stream);
    match read_frame(&mut r).map_err(|e| format!("bad hello: {e}"))? {
        Some(Message::Hello {
            protocol,
            map_hash,
            delay,
        }) => {
            if protocol != PROTOCOL {
                return Err(format!("protocol {protocol}, this host speaks {PROTOCOL}"));
            }
            if map_hash != identity.map_hash {
                return Err("a different map".into());
            }
            if delay != identity.delay {
                return Err(format!("delay {delay}, this match's is {}", identity.delay));
            }
            Ok(())
        }
        Some(other) => Err(format!("expected a hello, got {other:?}")),
        None => Err("closed before a hello".into()),
    }
}

/// Connect to a host, offer `identity`, and take the team it assigns.
/// `all_teams` is every team on the map; the host relays each of the
/// others' sets, so they are all this session's peers.
pub fn join(
    addr: impl ToSocketAddrs,
    identity: MatchIdentity,
    all_teams: &[TeamId],
) -> Result<Session, String> {
    let stream = TcpStream::connect(addr).map_err(|e| format!("cannot connect: {e}"))?;
    let _ = stream.set_nodelay(true);
    {
        let mut w = &stream;
        write_frame(
            &mut w,
            &Message::Hello {
                protocol: PROTOCOL,
                map_hash: identity.map_hash,
                delay: identity.delay,
            },
        )
        .map_err(|e| format!("cannot send hello: {e}"))?;
    }
    let me = {
        let mut r = BufReader::new(&stream);
        match read_frame(&mut r).map_err(|e| format!("bad welcome: {e}"))? {
            Some(Message::Welcome { team }) => team,
            Some(Message::Refuse { reason }) => return Err(format!("refused: {reason}")),
            Some(other) => return Err(format!("expected a welcome, got {other:?}")),
            None => return Err("the host closed before a welcome".into()),
        }
    };
    let peers: Vec<TeamId> = all_teams.iter().copied().filter(|t| *t != me).collect();
    let writer = Arc::new(Mutex::new(
        stream
            .try_clone()
            .map_err(|e| format!("cannot clone socket: {e}"))?,
    ));
    let (tx, rx) = channel();
    let lost_means = peers.clone();
    thread::spawn(move || {
        reader(
            stream,
            TeamId(u32::MAX),
            tx,
            Arc::new(Vec::new()),
            lost_means,
        )
    });
    Ok(Session {
        me,
        peers,
        links: vec![Link {
            team: TeamId(u32::MAX),
            writer,
        }],
        rx,
        lost: Vec::new(),
    })
}

impl Exchange for Session {
    fn send(&mut self, m: &Message) {
        for l in &self.links {
            write_to(&l.writer, m);
        }
    }

    fn poll(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(Event::Lost(t)) => {
                    if !self.lost.contains(&t) {
                        self.lost.push(t);
                        out.push(Event::Lost(t));
                    }
                }
                Ok(e) => out.push(e),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }

    fn peers(&self) -> Vec<TeamId> {
        self.peers.clone()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let bye = Message::Bye { sender: self.me };
        for l in &self.links {
            write_to(&l.writer, &bye);
            if let Ok(s) = l.writer.lock() {
                let _ = (&*s).flush();
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::{Command, CommandKind};
    use std::time::Duration;

    const ID: MatchIdentity = MatchIdentity {
        map_hash: 7,
        delay: 3,
    };

    /// Poll until `want` events arrive or the patience runs out.
    fn collect(s: &mut Session, want: usize) -> Vec<Event> {
        let mut got = Vec::new();
        for _ in 0..500 {
            got.extend(s.poll());
            if got.len() >= want {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        got
    }

    fn start_host(remote: Vec<TeamId>) -> (thread::JoinHandle<Session>, SocketAddr) {
        let (atx, arx) = channel();
        let h = thread::spawn(move || {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            drop(listener);
            atx.send(addr).unwrap();
            host(addr, TeamId(0), &remote, ID, |_| {}).unwrap().0
        });
        let addr = arx.recv().unwrap();
        (h, addr)
    }

    fn connect(addr: SocketAddr, teams: &[TeamId]) -> Session {
        let mut last = String::new();
        for _ in 0..200 {
            match join(addr, ID, teams) {
                Ok(s) => return s,
                Err(e) => last = e,
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("could not join: {last}");
    }

    #[test]
    fn a_host_relays_between_joiners_and_reports_a_lost_one() {
        let teams = [TeamId(0), TeamId(1), TeamId(2)];
        let (h, addr) = start_host(vec![TeamId(1), TeamId(2)]);
        let mut a = connect(addr, &teams);
        let mut b = connect(addr, &teams);
        let mut host = h.join().unwrap();
        assert_eq!(a.me, TeamId(1));
        assert_eq!(b.me, TeamId(2));
        assert_eq!(a.peers(), vec![TeamId(0), TeamId(2)]);
        assert_eq!(host.peers(), vec![TeamId(1), TeamId(2)]);

        let set = Message::Set {
            sender: TeamId(1),
            tick: 4,
            commands: vec![Command::new(4, TeamId(1), 0, CommandKind::SetSpeed(2))],
        };
        a.send(&set);
        assert_eq!(collect(&mut host, 1), vec![Event::Message(set.clone())]);
        assert_eq!(collect(&mut b, 1), vec![Event::Message(set)]);
        assert!(a.poll().is_empty(), "a set is not echoed to its sender");

        let hash = Message::Hash {
            sender: TeamId(0),
            tick: 4,
            hash: 99,
        };
        host.send(&hash);
        assert_eq!(collect(&mut a, 1), vec![Event::Message(hash.clone())]);
        assert_eq!(collect(&mut b, 1), vec![Event::Message(hash)]);

        drop(b);
        let host_saw = collect(&mut host, 2);
        assert!(
            host_saw.contains(&Event::Message(Message::Bye { sender: TeamId(2) })),
            "{host_saw:?}"
        );
        assert!(host_saw.contains(&Event::Lost(TeamId(2))), "{host_saw:?}");
        let a_saw = collect(&mut a, 1);
        assert!(
            a_saw.contains(&Event::Message(Message::Bye { sender: TeamId(2) })),
            "{a_saw:?}"
        );
    }

    #[test]
    fn a_joiner_on_another_map_is_refused_and_a_lost_host_loses_every_peer() {
        let teams = [TeamId(0), TeamId(1)];
        let (h, addr) = start_host(vec![TeamId(1)]);
        let wrong = MatchIdentity {
            map_hash: 8,
            delay: 3,
        };
        let mut refused = String::new();
        for _ in 0..200 {
            match join(addr, wrong, &teams) {
                Err(e) if e.contains("refused") => {
                    refused = e;
                    break;
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
                Ok(_) => panic!("joined on the wrong map"),
            }
        }
        assert!(refused.contains("a different map"), "{refused}");
        let mut a = connect(addr, &teams);
        let host = h.join().unwrap();
        drop(host);
        assert_eq!(
            collect(&mut a, 2),
            vec![
                Event::Message(Message::Bye { sender: TeamId(0) }),
                Event::Lost(TeamId(0))
            ]
        );
    }
}
