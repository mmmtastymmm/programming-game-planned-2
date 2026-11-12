*Closed record — see [../README.md](../README.md). Not spec.*

# T28 — What-if

Q55: *try in the fork* on each file window, beside deploy in a match and
alone on a replay, sending the file's deploys into the open fork for its
next tick, at any stop but one in step 4, a bundle that fails to load
refused with its error; *take it* deploying to the match the bundles the
last try sent, offered only while the tree still holds every file of them
as tried; refused in a live PvP match.

Completed in `07dafa5`, the merge of #120. A file window offers *try in
the fork* and *take it* while a fork is open, try alone on a replay and
neither in a live PvP match. A try is all or nothing, and sends only the
deployments the file reaches that are ahead of what the fork holds,
since an identical deploy would still restart their machines. What a
file's last try sent is kept by tree path and goes with the fork. A
take leaves out what the match already runs. A tried version is in no
log of the match, so the breakpoint catalog takes it in at the try, and
its version window, dots and frame marks work as any other's. `sim`'s
fork gained three readers on the way: whether a what-if can go in where
it stands, whether this one can, and the version a deployment holds. No
replay hash moved. The logic is tested headless; the two buttons were
not looked at on a screen before the merge, the build machine's display
being asleep. It was the last task of M8, the debugger, and fixes
[P5](../problems-fixed/problem-fixed-0005.md).
