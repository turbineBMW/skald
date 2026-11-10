#!/usr/bin/env bash
# User-local install: binary, .desktop, icons. No root needed.
set -euo pipefail
cd "$(dirname "$0")"

# Refuse to install a binary that can't build or can't play books. Each check
# records the Arch package that provides it.
missing=()
need_cmd() { command -v "$1" >/dev/null 2>&1 || missing+=("$2"); }
need_pc()  { pkg-config --exists "$1" 2>/dev/null || missing+=("$2"); }
need_gst() { gst-inspect-1.0 --exists "$1" 2>/dev/null || missing+=("$2"); }
# Build
need_cmd cargo             rust
need_cmd pkg-config        pkgconf
need_pc  gtk4              gtk4
need_pc  libadwaita-1      libadwaita
need_pc  webkitgtk-6.0     webkitgtk-6.0
need_pc  gstreamer-1.0     gstreamer
need_pc  gstreamer-play-1.0 gst-plugins-bad-libs
# Runtime: AAXC -> m4b remux, then GstPlay over MP4/AAC
need_cmd ffmpeg            ffmpeg
need_cmd gst-inspect-1.0   gstreamer
need_gst playbin           gst-plugins-base
need_gst audioconvert      gst-plugins-base
need_gst audioresample     gst-plugins-base
need_gst qtdemux           gst-plugins-good
need_gst aacparse          gst-plugins-good
need_gst scaletempo        gst-plugins-good
need_gst autoaudiosink     gst-plugins-good
need_gst avdec_aac         gst-libav
if ((${#missing[@]})); then
  pkgs=$(printf '%s\n' "${missing[@]}" | sort -u | paste -sd' ')
  echo "skald: missing dependencies, install them with:" >&2
  echo "  sudo pacman -S --needed $pkgs" >&2
  exit 1
fi

cargo build --release
install -Dm755 target/release/skald ~/.local/bin/skald
# Launchers often lack ~/.local/bin on PATH, and .desktop files don't expand ~,
# so bake the absolute binary path into the installed copy.
mkdir -p ~/.local/share/applications
sed "s#^Exec=.*#Exec=$HOME/.local/bin/skald %U#" data/dev.turbinebmw.Skald.desktop \
  > ~/.local/share/applications/dev.turbinebmw.Skald.desktop
chmod 644 ~/.local/share/applications/dev.turbinebmw.Skald.desktop
for n in 16 32 48 64 128 256 512; do
  install -Dm644 data/icons/skald-$n.png ~/.local/share/icons/hicolor/${n}x${n}/apps/dev.turbinebmw.Skald.png
done
# The scalable icon wraps the full-size art, so launchers that take an SVG over
# PNGs (Omarchy's does) draw it sharp at any size instead of an upscaled 16px PNG.
install -Dm644 data/icons/skald.svg ~/.local/share/icons/hicolor/scalable/apps/dev.turbinebmw.Skald.svg
gtk4-update-icon-cache -q -t -f ~/.local/share/icons/hicolor 2>/dev/null || true
update-desktop-database ~/.local/share/applications 2>/dev/null || true
echo "installed: ~/.local/bin/skald"
