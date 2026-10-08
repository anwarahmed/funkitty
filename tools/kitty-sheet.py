#!/usr/bin/env python3
"""Shows Pink Kitty drawn several ways, to choose how she looks in the game.
This is the sheet she was chosen from ("1C"); src/kitty.rs is how the game draws her.

Run it in the terminal you play in:   python3 tools/kitty-sheet.py
First row: four ways to wear the scarf (1 to 4). Second row: three faces (A to C).
Any scarf goes with any face, so the answer is a pair such as "2A".

    python3 tools/kitty-sheet.py 2A 4C       draw just those combinations
    python3 tools/kitty-sheet.py --ppm FILE  write the whole sheet as an image instead
"""
import shutil, sys

W, H = 44, 48                    # one kitty, in pixels (a terminal cell is 1 x 2 of them)
CX = 21.5                        # she is symmetrical about this line

WHITE, SHADE = (255, 255, 255), (228, 224, 238)
LINE, WHISKER = (96, 72, 96), (186, 174, 194)
PINK, PINK_DARK, PINK_LIGHT = (255, 186, 210), (238, 138, 176), (255, 218, 232)
EAR, NOSE, BLUSH, EYE = (255, 168, 196), (250, 110, 150), (255, 200, 216), (52, 40, 62)


def ell(cx, cy, rx, ry):
    return lambda x, y: ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1


def tri(a, b, c):
    def side(p, q, x, y):
        return (x - q[0]) * (p[1] - q[1]) - (p[0] - q[0]) * (y - q[1])

    def inside(x, y):
        d = side(a, b, x, y), side(b, c, x, y), side(c, a, x, y)
        return not (min(d) < 0 and max(d) > 0)
    return inside


def box(x0, y0, x1, y1):
    return lambda x, y: x0 <= x <= x1 and y0 <= y <= y1


def both(f, g):
    return lambda x, y: f(x, y) and g(x, y)


def either(*fs):
    return lambda x, y: any(f(x, y) for f in fs)


def without(f, g):
    return lambda x, y: f(x, y) and not g(x, y)


def mirror(f):
    return lambda x, y: f(2 * CX - x, y)


class Canvas:
    def __init__(self):
        self.px = {}

    def paint(self, shape, color, line=LINE):
        """Fills a shape, with a one pixel line around it."""
        pts = {(x, y) for y in range(H) for x in range(W) if shape(x, y)}
        if line:
            for x, y in pts:
                for p in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                    if p not in pts and 0 <= p[0] < W and 0 <= p[1] < H:
                        self.px[p] = line
        for p in pts:
            self.px[p] = color

    def pair(self, shape, color, line=LINE):
        self.paint(either(shape, mirror(shape)), color, line)

    def tint(self, shape, color, only):
        """Recolors the pixels of a shape that are currently one given color."""
        for y in range(H):
            for x in range(W):
                if shape(x, y) and self.px.get((x, y)) == only:
                    self.px[(x, y)] = color

    def dots(self, color, *pts):
        for x, y in pts:
            self.px[(x, y)] = color
            self.px[(W - 1 - x, y)] = color


HEAD = ell(CX, 20, 18, 14.5)
HOOD = ell(CX, 19.5, 19.5, 16)
OPENING = ell(CX, 21.5, 14.5, 11.5)


def body(c):
    c.paint(ell(34, 41, 3, 5), WHITE)                           # tail
    c.paint(ell(CX, 41, 9.5, 7), WHITE)
    c.tint(without(ell(CX, 41, 9.5, 7), ell(CX, 39.5, 9, 6.5)), SHADE, WHITE)
    c.pair(ell(16.5, 46.6, 3.2, 1.5), WHITE)                    # feet


def ears(c, color=WHITE, inner=True, lift=0):
    c.pair(tri((5, 15 - lift), (7, 1 - lift), (18, 7 - lift)), color)
    if inner:
        c.pair(tri((8, 10 - lift), (9, 4 - lift), (14, 8 - lift)), EAR, None)


def head(c):
    c.paint(HEAD, WHITE)
    c.tint(without(HEAD, ell(CX, 19, 17.5, 14)), SHADE, WHITE)


def folds(c, shape):
    """A darker rim along the bottom of a piece of scarf and a lighter one on top."""
    lifted = lambda x, y: shape(x, y - 1)
    lowered = lambda x, y: shape(x, y + 1)
    c.tint(without(shape, lowered), PINK_DARK, PINK)
    c.tint(without(shape, lifted), PINK_LIGHT, PINK)


def neck(c, tails=True):
    wrap = ell(CX, 35.5, 11.5, 3.2)
    if tails:
        end = either(box(26, 36, 30, 43), ell(28, 43.5, 2.5, 1.5))
        c.paint(end, PINK)
        c.tint(box(26, 42, 30, 42), PINK_DARK, PINK)
    c.paint(wrap, PINK)
    folds(c, wrap)


def scarf_hood(c):
    """1: a snug hood. Her ears are tucked inside it."""
    body(c)
    ears(c, PINK, inner=False)
    c.paint(HOOD, PINK)
    c.tint(without(HOOD, ell(CX, 18.5, 19, 15.5)), PINK_DARK, PINK)
    c.tint(without(ell(CX, 17, 17, 13), ell(CX, 18, 17, 13)), PINK_LIGHT, PINK)
    c.paint(OPENING, WHITE)
    neck(c)
    return 22, None


def scarf_ears_out(c):
    """2: a headscarf tied in a bow under her chin. Her ears poke out."""
    body(c)
    c.paint(HOOD, PINK)
    c.tint(without(HOOD, ell(CX, 18.5, 19, 15.5)), PINK_DARK, PINK)
    ears(c, lift=1)
    c.tint(without(ell(CX, 17, 17, 13), ell(CX, 18, 17, 13)), PINK_LIGHT, PINK)
    c.paint(OPENING, WHITE)
    c.pair(tri((20, 36), (12, 32.5), (12, 39.5)), PINK)         # the bow
    c.dots(PINK_DARK, (14, 36), (15, 36), (16, 36), (17, 36))
    c.paint(ell(CX, 36, 2, 2), PINK)
    return 22, None


def scarf_band(c):
    """3: a band over her head and a fluffy scarf round her neck. Most of her shows."""
    body(c)
    ears(c)
    head(c)
    band = both(HEAD, without(ell(CX, 8, 13, 6.5), ell(CX, 16, 16, 5.5)))
    c.paint(band, PINK)
    folds(c, band)
    neck(c)
    return 21, 3


def scarf_drape(c):
    """4: a loose shawl over her head, hanging down both sides."""
    drape = either(ell(CX, 21, 21, 18.5), box(1, 21, 8, 43), box(35, 21, 42, 43))
    c.paint(drape, PINK)
    c.tint(either(box(4, 30, 4, 43), box(39, 30, 39, 43)), PINK_DARK, PINK)
    c.tint(either(box(1, 43, 8, 43), box(35, 43, 42, 43)), PINK_DARK, PINK)
    body(c)
    ears(c, lift=1)
    head(c)
    cover = without(HEAD, ell(CX, 23.5, 16.5, 14))
    c.paint(cover, PINK)
    c.tint(without(cover, lambda x, y: cover(x, y - 1)), PINK_LIGHT, PINK)
    wrap = ell(CX, 35.5, 12.5, 2.8)
    c.paint(wrap, PINK)
    folds(c, wrap)
    return 22, 6


def face(c, fy, whisker_x, eyes):
    nose_y = fy + 4
    if eyes == "A":      # big and sparkly
        c.pair(ell(13.5, fy + 0.5, 2.2, 2.8), EYE, None)
        for x in (12, 28):   # the light catches both eyes from the same side
            for p in ((x, fy - 1), (x + 1, fy - 1), (x, fy), (x + 3, fy + 2)):
                c.px[p] = WHITE
    elif eyes == "B":    # small and simple
        c.pair(ell(13.5, fy + 1, 1.2, 1.8), EYE, None)
    else:                # smiling
        c.dots(EYE, (11, fy + 2), (12, fy + 1), (13, fy), (14, fy), (15, fy + 1), (16, fy + 2))
    c.pair(ell(10.5, fy + 5.5, 2.2, 1.2), BLUSH, None)
    c.dots(NOSE, (21, nose_y), (20, nose_y), (21, nose_y + 1))
    c.dots(LINE, (21, nose_y + 2), (20, nose_y + 3), (19, nose_y + 3), (18, nose_y + 2))
    if whisker_x is not None:
        c.dots(WHISKER, *[(whisker_x + i, fy + 2) for i in range(4)])
        c.dots(WHISKER, *[(whisker_x + i, fy + 4 + (i < 2)) for i in range(4)])


SCARVES = {"1": scarf_hood, "2": scarf_ears_out, "3": scarf_band, "4": scarf_drape}
NAMES = {"1": "snug hood", "2": "ears out, bow", "3": "band + neck scarf", "4": "loose shawl",
         "A": "sparkly eyes", "B": "simple eyes", "C": "smiling eyes"}


def kitty(scarf, eyes):
    c = Canvas()
    fy, whisker_x = SCARVES[scarf](c)
    face(c, fy, whisker_x, eyes)
    return c.px


def cells(px):
    """The picture as lines of half blocks, each cell carrying its two pixels."""
    lines = []
    for y in range(0, H, 2):
        out = []
        for x in range(W):
            top, bottom = px.get((x, y)), px.get((x, y + 1))
            if top is None and bottom is None:
                out.append("\x1b[0m ")
            elif bottom is None:
                out.append("\x1b[0;38;2;%d;%d;%dm▀" % top)
            elif top is None:
                out.append("\x1b[0;38;2;%d;%d;%dm▄" % bottom)
            else:
                out.append("\x1b[38;2;%d;%d;%d;48;2;%d;%d;%dm▀" % (top + bottom))
        lines.append("".join(out) + "\x1b[0m")
    return lines


def show(row):
    """Prints some (label, picture) pairs side by side, as many as the window fits."""
    per_line = max(1, (shutil.get_terminal_size().columns + 2) // (W + 2))
    for i in range(0, len(row), per_line):
        part = row[i:i + per_line]
        for lines in zip(*[cells(px) for _, px in part]):
            print("  ".join(lines))
        print("  ".join("\x1b[1m" + label.center(W) + "\x1b[0m" for label, _ in part))
        print()


def ppm(path, rows, scale=6, back=(30, 30, 46)):
    wide = max(len(r) for r in rows)
    img = [[back] * (wide * (W + 4) * scale) for _ in range(len(rows) * (H + 4) * scale)]
    for j, row in enumerate(rows):
        for i, (_, px) in enumerate(row):
            for (x, y), color in px.items():
                for dy in range(scale):
                    for dx in range(scale):
                        img[(j * (H + 4) + 2 + y) * scale + dy][(i * (W + 4) + 2 + x) * scale + dx] = color
    with open(path, "wb") as f:
        f.write(b"P6 %d %d 255\n" % (len(img[0]), len(img)))
        f.write(bytes(v for line in img for color in line for v in color))


def main():
    args = sys.argv[1:]
    out = args[args.index("--ppm") + 1] if "--ppm" in args else None
    picks = [a.upper() for a in args if len(a) == 2 and a[0] in SCARVES and a[1].upper() in "ABC"]
    if picks:
        rows = [[(p, kitty(p[0], p[1])) for p in picks]]
    else:
        rows = [[("%s  %s" % (s, NAMES[s]), kitty(s, "A")) for s in SCARVES],
                [("%s  %s" % (e, NAMES[e]), kitty("3", e)) for e in "ABC"]]
    if out:
        return ppm(out, rows)
    print()
    for row in rows:
        show(row)
    if not picks:
        print("Pick a scarf (1-4) and a face (A-C), for example 2A. To see one: python3 tools/kitty-sheet.py 2A\n")


main()
