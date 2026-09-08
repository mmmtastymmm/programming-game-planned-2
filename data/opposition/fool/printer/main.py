# The Fool's printer: print red bots while under the cap, and otherwise wait.
# This is shipped source (design-invariant DI8): it runs under docs/01 exactly
# as a player's program does.

while True:
    try:
        print("red")
    except ValueError:
        wait(10)
