#!/usr/bin/env bash
# Needs the wasm32-unknown-unknown target and a wasm-bindgen CLI at the version in Cargo.lock.
set -euo pipefail

BUDGET_BYTES=200000
root="$(cd "$(dirname "$0")/.." && pwd)"
dist="$root/web/dist"
wasm="$root/target/wasm32-unknown-unknown/web/little_quest_web.wasm"

want="$(awk '/^name = "wasm-bindgen"$/ {getline; gsub(/[^0-9.]/, ""); print; exit}' "$root/Cargo.lock")"
have="$( (wasm-bindgen --version 2>/dev/null || true) | awk '{print $2}')"
if [ "$want" != "$have" ]; then
  echo "wasm-bindgen CLI is ${have:-missing} but Cargo.lock has $want. Run: cargo install wasm-bindgen-cli --version =$want --locked" >&2
  exit 1
fi

cd "$root"
cargo build --profile web -p little-quest-web --target wasm32-unknown-unknown

rm -rf "$dist"
mkdir -p "$dist"
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section --out-dir "$dist" --out-name little_quest_web "$wasm"
cp web/static/index.html web/static/app.js "$dist/"

if [ -n "${WASM_OPT:-}" ]; then
  wasm-opt -Oz "$dist/little_quest_web_bg.wasm" -o "$dist/little_quest_web_bg.wasm"
fi

version="$(cat "$dist/little_quest_web.js" "$dist/little_quest_web_bg.wasm" "$dist/app.js" "$dist/index.html" | shasum -a 256 | cut -c1-10)"
for f in index.html app.js; do
  sed "s/__VERSION__/$version/g" "$dist/$f" > "$dist/$f.tmp"
  mv "$dist/$f.tmp" "$dist/$f"
done
if grep -rl "__VERSION__" "$dist" >/dev/null; then
  echo "unstamped __VERSION__ left in dist" >&2
  exit 1
fi

total=0
for f in index.html app.js little_quest_web.js little_quest_web_bg.wasm; do
  raw="$(wc -c < "$dist/$f" | tr -d ' ')"
  gz="$(gzip -9 -c "$dist/$f" | wc -c | tr -d ' ')"
  printf '%-26s %8s raw %8s gzip\n' "$f" "$raw" "$gz"
  total=$((total + gz))
done
echo "version $version, total $total bytes gzipped, budget $BUDGET_BYTES"
if [ "$total" -gt "$BUDGET_BYTES" ]; then
  echo "over budget by $((total - BUDGET_BYTES)) bytes" >&2
  exit 1
fi
