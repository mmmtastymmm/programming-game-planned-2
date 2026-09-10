# The showcase fixture: one program through as much of the language as one
# transcript can carry. Its transcript hash is pinned; a change here or in the
# interpreter moves it, and the PR says why (CLAUDE.md).
from geometry import dist2, Point
import geometry

class Machine:
    kind = "machine"

    def __init__(self, name, hp):
        self.name = name
        self.hp = hp
        self.trail = []

    def __str__(self):
        return f"{self.kind}:{self.name}({self.hp})"

    def __lt__(self, other):
        return (self.hp, self.name) < (other.hp, other.name)

    def __eq__(self, other):
        return isinstance(other, Machine) and self.name == other.name

    def hit(self, n):
        self.hp -= n
        self.trail.append(n)
        return self.hp

class Bot(Machine):
    kind = "bot"

    def __len__(self):
        return len(self.trail)

class Blocked(ValueError):
    def __init__(self, where):
        self.where = where

fleet = [Bot("b" + str(i), 10 - i) for i in range(5)]
fleet.append(Machine("m", 3))
for m in fleet:
    m.hit(m.hp % 3)
log(sorted(fleet), min(fleet), max(fleet), fleet[0] in fleet, Machine("b1", 0) in fleet)
log([len(b) for b in fleet if isinstance(b, Bot)], any(fleet), all(fleet))

acc = {}
seen = set()
for i in range(40):
    k = f"{(i * 7919) % 23:02d}"
    acc[k] = acc.get(k, 0) + (i ** 3) / 7
    seen.add((i % 5, k))
log(sorted(acc.items(), key=lambda kv: (-kv[1], kv[0]))[:3], len(seen), sorted(seen)[:2])

xs = [(-1) ** i * (10 ** 15 + i) / 3 for i in range(9)]
xs.sort(reverse=True)
log(xs, sum(xs), f"{xs[0]:.3f}|{xs[-1]:,}|{7:04d}|{'ab':^6}|")
log(dist2(Point(1, 2), Point(4, 6)), geometry.origin, geometry.origin.x, str(geometry))

def classify(v):
    match v:
        case 0 | 1 | 2 as small:
            return f"small {small}"
        case [first, *rest] if len(rest) > 1:
            return f"seq {first} {rest}"
        case {"kind": k, **extra}:
            return f"map {k} {extra}"
        case Bot(name=n, hp=hp):
            return f"bot {n} {hp}"
        case Machine(hp=hp):
            return f"machine {hp}"
        case str() | num():
            return f"scalar {v}"
        case _:
            return "other"

for v in [1, [1, 2, 3], {"kind": "x", "n": 2}, fleet[1], fleet[-1], "s", 9.5, None]:
    log(classify(v))

def risky(k):
    try:
        if k == 0:
            return 1 / 0
        if k == 1:
            raise Blocked((2, 3))
        return "fine"
    except ZeroDivisionError as e:
        return ("zd", e.line)
    except Blocked as e:
        return ("blocked", e.where, e.args)
    finally:
        log("finally", k)

for k in range(3):
    log(risky(k))

t = wait()
log("waited", t)
text = "the quick brown fox"
log(text.upper().split(), "-".join(sorted(set(text.replace(" ", "")))), text.find("brown"), text[4:9], text[::-2])
big = 10 ** 20
log(big * 3, big // 7, big % 7, -big / 3, round(2.5), round(-1.5), round(1 / 3, 4), int(-2.9), num("12.5"))
try:
    log(big * big)
except OverflowError as e:
    log("overflow", e.line)
log(fleet[2].hit(100), fleet[2])
x = fleet[2].nothing
