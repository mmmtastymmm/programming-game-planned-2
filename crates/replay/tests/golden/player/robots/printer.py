# The fixture's player printer: printers print by themselves (Q42), so it
# logs its print's countdown and how many machines its team sees — own and
# rival, bots and buildings — so a change in the printing rule moves the
# transcript.
while True:
    log("printing", me().printing, "sees", len(see()))
    wait(5)
