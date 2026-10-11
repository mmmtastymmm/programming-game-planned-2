# The fixture's player printer: printers print by themselves (Q42), so it
# logs its print's countdown and the team's bots it can see, so a change in
# the printing rule moves the transcript.
while True:
    log("printing", me().printing, "sees", len(see()))
    wait(5)
