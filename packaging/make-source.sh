#!/usr/bin/env bash
# The source package for omarchy-mobile (PKGBUILD's source): this tree with
# the omarchy-mobile kit's crates copied in place of the kit/ link, and the
# crates vendored so the phone builds offline-capable and pinned.
#   packaging/make-source.sh  ->  packaging/skald-src.tar.gz
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=packaging/skald-src.tar.gz
STAGE=$(mktemp -d); trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/skald"
git ls-files -z | grep -zv '^kit$' | xargs -0 cp --parents -t "$STAGE/skald"
mkdir -p "$STAGE/skald/kit"
cp -rL kit/omarchy-mobile kit/omarchy-theme "$STAGE/skald/kit/"
rm -rf "$STAGE/skald/kit/"*/target
( cd "$STAGE/skald" && mkdir -p .cargo && cargo vendor --locked vendor > .cargo/config.toml )
git describe --always --dirty > "$STAGE/skald/packaging/version"
tar -C "$STAGE" -czf "$OUT" skald
echo "$OUT ($(du -h "$OUT" | cut -f1))"
