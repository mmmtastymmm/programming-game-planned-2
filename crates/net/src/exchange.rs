//! The driver's view of the other peers (`docs/06`, The driver): it sends
//! its sets and hashes, and polls for theirs. What arrives is data for the
//! log; the exchange never touches the sim. [`Loopback`] is a pair of ends
//! joined in memory, for driving two peers in one test; [`crate::tcp`] is
//! the transport a match uses.

use crate::wire::Message;
use sim::TeamId;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

/// What the exchange hands the driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Message(Message),
    /// The connection carrying `TeamId`'s sets is gone; nothing more will
    /// arrive from it.
    Lost(TeamId),
}

pub trait Exchange {
    /// Send to every other peer. A failure surfaces later as [`Event::Lost`]
    /// from the reader that notices the connection is gone.
    fn send(&mut self, m: &Message);
    /// Everything that arrived since the last poll, in arrival order.
    fn poll(&mut self) -> Vec<Event>;
    /// The peers this exchange carries, in team order.
    fn peers(&self) -> Vec<TeamId>;
}

/// One end of an in-memory pair: what one end sends, the other polls.
pub struct Loopback {
    me: TeamId,
    other: TeamId,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    lost: bool,
}

impl Loopback {
    /// Two ends joined: the first is team `ta`'s and sees team `tb`, and
    /// the reverse.
    pub fn pair(ta: TeamId, tb: TeamId) -> (Loopback, Loopback) {
        let (atx, brx) = channel();
        let (btx, arx) = channel();
        (
            Loopback {
                me: ta,
                other: tb,
                tx: atx,
                rx: arx,
                lost: false,
            },
            Loopback {
                me: tb,
                other: ta,
                tx: btx,
                rx: brx,
                lost: false,
            },
        )
    }

    pub fn me(&self) -> TeamId {
        self.me
    }
}

impl Exchange for Loopback {
    fn send(&mut self, m: &Message) {
        let _ = self.tx.send(Event::Message(m.clone()));
    }

    fn poll(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(e) => out.push(e),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if !self.lost {
                        self.lost = true;
                        out.push(Event::Lost(self.other));
                    }
                    break;
                }
            }
        }
        out
    }

    fn peers(&self) -> Vec<TeamId> {
        vec![self.other]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loopback_delivers_in_order_and_reports_the_dropped_end_once() {
        let (mut a, mut b) = Loopback::pair(TeamId(0), TeamId(1));
        a.send(&Message::Bye { sender: TeamId(0) });
        a.send(&Message::Hash {
            sender: TeamId(0),
            tick: 1,
            hash: 5,
        });
        assert_eq!(
            b.poll(),
            vec![
                Event::Message(Message::Bye { sender: TeamId(0) }),
                Event::Message(Message::Hash {
                    sender: TeamId(0),
                    tick: 1,
                    hash: 5
                }),
            ]
        );
        assert_eq!(b.poll(), vec![]);
        assert_eq!(a.me(), TeamId(0));
        drop(a);
        assert_eq!(b.poll(), vec![Event::Lost(TeamId(0))]);
        assert_eq!(b.poll(), vec![]);
        assert_eq!(b.peers(), vec![TeamId(0)]);
    }
}
