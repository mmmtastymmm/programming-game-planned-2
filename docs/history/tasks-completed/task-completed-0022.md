*Closed record — see [../README.md](../README.md). Not spec.*

# T22 — Bot deployments become numbers, the color derived, the badge on the bot

Q40 built. In `sim`: a team's bot deployments are `"1"`, `"2"`, … one per
printer without bound, created on unlock or on a deploy to any positive
number, which waits if locked; `print` takes a positive integral `num`; the
`deployment` attribute is a `num` for a bot; a bot is named `<n>-<id>`; the
snapshot lists the unlocked deployments and any holding or awaiting a
bundle. The shipped programs moved from `red` to `1` and `print("red")` to
`print(1)`, and the golden replay fixture regenerated — every deployment
name, bot name and print argument in the scenario changed. In `render`: a
bot's atlas is the paint color its number selects, cycling; its number is
drawn over its body; the editor orders deployments numerically.

Completed in `2bce195`.
