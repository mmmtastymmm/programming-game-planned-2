*Closed record — see [../README.md](../README.md). Not spec.*

# T18 — Floating windows over the map in `render`

Q39 built: the map fills the window; the time bar is the one fixed strip
and doubles as the dock, a toggle per window; one `egui` window per
deployment's program holds what the editor's panel held, and the inspector,
the tools and the log are windows too, each dragged, resized, collapsed and
closed, several programs open at once. The first color deployment opens by
default. Each window's placement and whether it is open are saved to
`windows.toml` beside the programs at most once a second when they change
and restored at start; a missing or malformed file is the default layout.
Nothing about it reaches a peer.

With T18 closed, milestone M5 — the interface — is finished as far as its
rulings go; Q36 through Q38 are open and each will bring its own task.

Completed in `74593ea`.
