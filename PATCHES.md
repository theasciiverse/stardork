# Changes from upstream

This is the Asciiverse packaging of StarDork, Ryan Kulla's ncurses game
(sourceforge.net/projects/stardork). No upstream repository is published:
history starts from the 0.7 release tarball of 2011-07-25 (tag `0.7`,
branch `master`); everything after it is ours, on branch `asciiverse`.

## Licence

The tarball carries no licence file or notice. The SourceForge project
page gives the licence as "GNU General Public License version 2.0
(GPLv2)", so `COPYING` is the GPL version 2 text (from gnu.org) and the
SPDX id is `GPL-2.0-only`.

## The Rust port (`src/main.rs`)

`stardork.c` stays as it is; the catalog ships `stardork`, the same game
ported to one Rust file (crossterm), so it is one binary on Linux, macOS
and Windows and needs no ncurses. The binary keeps upstream's name.

Kept: the screen and status line where they were (the ship starts at
row 18, column 45), the keys, the stars (one batch of one per column at
the start and after every wormhole), the probes (5 ms a cell, diagonals
two columns per row, passing through stars), power, lives and levels
(9, 3, a win on reaching level 30), a star under the ship costing power
and turning the screen red until a key is pressed, once per star, every
key counting as a move, the GAME OVER animation and the winner's screen.
The playfield is the terminal's size at start, as before.

Changed:

- **Number keys 1 to 9 move**, as the README promises; the original only
  read the keypad's arrow, Home, PgUp, End and PgDn codes, so numbers
  worked only with Num Lock off. Those keys still work.
- **q and Esc quit** as well as Ctrl+C, with the same "Bye Bye!"; fire
  keys work in capitals too.
- **No sound.** The original beeped through the Linux console's
  `KIOCSOUND` ioctl, which is silent in terminal emulators and does not
  exist on macOS or Windows.
- **Every wormhole can be reached.** The original could put one on the
  status line, a row below the screen or a column left of it; now each
  lands on a free cell of the playfield, never on a star.
- **The ship stays visible** when it sits on a star; the original drew
  the star over it.
- **The power bar** blanks every unit lost; the original left one behind
  when two stars hit at once, and drew a stray star in the top-left
  corner.
- **Any key** leaves the GAME OVER screen (the original wanted Enter).
- **The winner's name** comes from `LOGNAME`, then `USER` or `USERNAME`
  (Windows).
- **A terminal smaller than 64x20** gets a message instead of a broken
  screen; the original had no check. No limits of 255 columns or levels.
- **Colours** go out as the plain codes curses sends (37 white, 31 red).
- **`--seed N`** makes a game repeatable (used for the catalog preview);
  `--version` prints the version.
