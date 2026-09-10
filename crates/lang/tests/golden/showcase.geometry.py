class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def __str__(self):
        return f"({self.x}, {self.y})"

def dist2(a, b):
    dx = a.x - b.x
    dy = a.y - b.y
    return dx * dx + dy * dy

origin = Point(0, 0)
