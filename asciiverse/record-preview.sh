#!/bin/sh
# Records the 100x30 catalog preview, asciiverse/stardork.cast
# (hub/RECORDING.md), with hub's castrec.py. asciiverse/bot.py first plays
# seed 1 in tmux and lists its keys (151 moves, every wormhole, three
# stars hit); the same keys are then replayed at watching speed into a
# fresh game, which ends on the winner's screen. Needs tmux.
#   CASTREC=path/to/hub/tools/castrec.py asciiverse/record-preview.sh
set -e
cd "$(dirname "$0")/.."
here=$(pwd)
castrec=${CASTREC:-$(cd ../../hub/tools && pwd)/castrec.py}
cargo build --release
keys=$(python3 -I asciiverse/bot.py "$here/target/release/stardork" 1 250 2>/dev/null)
script="w1.5"
IFS=';'
for k in $keys; do
  case $k in
    k[fasdzxcv]) script="$script;$k;w0.6" ;;   # a probe flies
    kspace) script="$script;w0.7;$k;w0.2" ;;    # the red screen, then a key
    *) script="$script;$k;w0.22" ;;
  esac
done
unset IFS
script="$script;w6;kspace;w0.5"
tmp=$(mktemp -d)
cp target/release/stardork "$tmp/"
(cd "$tmp" && PATH="$tmp:$PATH" HOME="$tmp" \
  python3 "$castrec" "$here/asciiverse/stardork.cast" . "$script" -- stardork --seed 1)
rm -rf "$tmp"
