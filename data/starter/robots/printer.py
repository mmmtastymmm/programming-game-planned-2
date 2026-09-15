# A starter printer: print deployment 1's bots while under the cap, and otherwise wait.
# Edit this file and press Shift+D in the game to redeploy it (docs/02, docs/06).

while True:
    try:
        print(1)
    except ValueError:
        wait(10)
