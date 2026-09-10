# A starter printer: print red bots while under the cap, and otherwise wait.
# Edit this file and press D in the game to redeploy it (docs/02, docs/06).

while True:
    try:
        print("red")
    except ValueError:
        wait(10)
