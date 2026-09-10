# The fixture's player printer: prints red bots while it can, and logs the
# team's cap so a change in Q24's arithmetic moves the transcript.
while True:
    try:
        print("red")
    except ValueError:
        log("cannot print", len(see()))
        wait(5)
