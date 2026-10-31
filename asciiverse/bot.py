#!/usr/bin/env python3
"""bot.py BINARY SEED TURNS: plays StarDork in a 100x30 tmux pane and prints
the keys it pressed, one castrec step per key, for record-preview.sh.

Each turn it reads the screen, and if the wormhole (@) is in line with the
ship (X) (same row or column, or a diagonal, which moves two columns per
row) it fires a probe; otherwise it takes the first step of the shortest
path around the stars (.) to a cell in line with the wormhole. The game is
turn-based and seeded, so replaying the same keys replays the same game."""
import collections, subprocess, sys, time

binary, seed, turns = sys.argv[1], sys.argv[2], int(sys.argv[3])
S = "stardork-bot"
W, H = 100, 30
MOVES = {(-1, 0): "up", (1, 0): "down", (0, -1): "left", (0, 1): "right",
         (-1, -1): "7", (-1, 1): "9", (1, -1): "1", (1, 1): "3"}
TMUX = {"up": "Up", "down": "Down", "left": "Left", "right": "Right"}
SHOTS = {(0, 1): "f", (0, -1): "a", (-1, 0): "s", (1, 0): "d",
         (-1, -2): "z", (-1, 2): "x", (1, -2): "c", (1, 2): "v"}

def tmux(*a): return subprocess.run(["tmux", *a], capture_output=True, text=True).stdout
def screen(): return [l.ljust(W) for l in (tmux("capture-pane", "-p", "-t", S).split("\n") + [""] * H)[:H]]
def find(rows, ch): return next(((y, x) for y in range(1, H) for x in range(W) if rows[y][x] == ch), None)

def shot(ship, hole):
    dy, dx = hole[0] - ship[0], hole[1] - ship[1]
    for (sy, sx), k in SHOTS.items():
        n = dy // sy if sy else dx // sx
        if n > 0 and (sy * n, sx * n) == (dy, dx):
            return k
    return None

def step(rows, ship, hole):
    prev, todo = {ship: None}, collections.deque([ship])
    while todo:
        c = todo.popleft()
        if c != ship and shot(c, hole):
            while prev[c] != ship:
                c = prev[c]
            return MOVES[(c[0] - ship[0], c[1] - ship[1])]
        for d in MOVES:
            n = (c[0] + d[0], c[1] + d[1])
            if 1 <= n[0] < H and 0 <= n[1] < W and n not in prev and rows[n[0]][n[1]] not in ".@":
                prev[n] = c
                todo.append(n)
    return "f"

tmux("kill-session", "-t", S)
tmux("new-session", "-d", "-s", S, "-x", str(W), "-y", str(H), f"{binary} --seed {seed}; sleep 5")
time.sleep(0.5)
keys = []
for _ in range(turns):
    if "\x1b[31m" in tmux("capture-pane", "-p", "-e", "-t", S):
        # red: a star hit the ship, and the game waits for a key
        keys.append("space")
        tmux("send-keys", "-t", S, "Space")
        time.sleep(0.15)
        continue
    rows = screen()
    ship, hole = find(rows, "X"), find(rows, "@")
    if not ship or not hole:
        print("\n".join(rows), file=sys.stderr)
        break
    k = shot(ship, hole) or step(rows, ship, hole)
    keys.append(k)
    tmux("send-keys", "-t", S, TMUX.get(k, k))
    time.sleep(0.6 if k in "fasdzxcv" else 0.08)
print(screen()[0].strip(), file=sys.stderr)
tmux("kill-session", "-t", S)
print(";".join("k" + k for k in keys))
