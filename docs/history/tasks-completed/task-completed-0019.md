*Closed record — see [../README.md](../README.md). Not spec.*

# T19 — The start and end screens, and the saved replay

Q36 built. In `render` the app runs as three states — start, match, end —
in one window. The start screen lists `data/maps` and `data/opposition`,
seats any number of cards in team order after the player's, takes the
programs directory, and offers play, host or join; host and join
handshake on a thread of their own (the session crosses threads, the sim
does not), so the window stays live while it waits, and a failed
handshake or a missing map is shown on the screen. The end screen sits
over the frozen map with the winner or the draw, the ending tick, the
final hash and the replay's path; *again* returns to the start, the last
match's entities despawning as it is entered, and *quit* leaves.

A match that ends or desyncs writes `replays/<map>-<tick>-<hash>.ron`
inside the programs directory in `crates/replay`'s RON, so `render` now
depends on `replay`, and `06` draws the edge. Every flag still skips the
screen; `--play`, which chooses nothing, starts the defaults at once, for
testing. Nothing touched a hash.

Left for later: a host waiting on a peer has no cancel but *quit*, and a
desync goes straight to the end screen until T21 puts its banner first.

Completed in `7c5ff4a`.
