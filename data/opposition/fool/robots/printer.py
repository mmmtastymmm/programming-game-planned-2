# The Fool's printer: it prints by itself, taking turns with the team's other
# printers until the team is at its cap (Q42), so its program only waits.
# This is shipped source (design-invariant DI8): it runs under docs/01 exactly
# as a player's program does.

while True:
    wait(60)
