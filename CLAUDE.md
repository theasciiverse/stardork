# StarDork (Asciiverse packaging)

Ryan Kulla's StarDork 0.7 (C, ncurses, 2011), packaged for the Asciiverse
catalog (id `stardork`, kind `game`) as a one-file Rust port with
crossterm, `src/main.rs`, like Ski! and Terminal Rain. Not our game: keep
the rules as they are and list every change in PATCHES.md.

- No upstream repository: branch `master` is the 0.7 tarball (tag `0.7`);
  ours is branch `asciiverse`, `origin` github.com/theasciiverse/stardork.
- Licence: GPL-2.0-only, from the SourceForge project page (the tarball
  has none); `COPYING` is the GPLv2 text.
- `stardork.c` stays untouched, as the reference.
- Release: tag `v0.7-av.<n>` and push it; `.github/workflows/release.yml`
  builds Linux musl, macOS and Windows.
- `Screen` keeps a copy of the terminal and sends only changed cells, as
  curses does; without it every probe step redraws every star (a 13 MB
  preview).
- Preview: `CASTREC=…/hub/tools/castrec.py asciiverse/record-preview.sh`
  (needs tmux): `asciiverse/bot.py` plays seed 1 to the win, 151 moves,
  and the keys are replayed into the recording, about 58 s.
