# Skald

*Skald* (Old Norse *skáld*) — a poet and storyteller in the Viking-age Norse courts, who
composed and recited verse from memory. A fitting name for an app whose job is to read you stories.

A native GTK4/libadwaita Audible client for Linux that **keeps Whispersync working** —
it speaks the same `api.audible.com` protocol as the mobile apps, so pausing on the
laptop and resuming on the phone lands at the same spot.

## Omarchy themes

When an Omarchy theme is present, Skald takes its window colors, accent, and
light/dark mode from the active palette and follows `omarchy theme set` live.
It derives libadwaita colors from `colors.toml` by default; a theme can take
full control by including a `skald.css` file.

## Layout

- `src/auth/`   — Amazon device registration, token storage, ADP request signing
- `src/api/`    — typed client: library, last-positions (read/write), license/AAXC
- `src/player/` — GStreamer playback, MPRIS, periodic position push
- `src/ui/`     — libadwaita UI; WebKitGTK only for the one-time sign-in page
- `probe/`      — Python scripts (mkb79/audible) used to verify endpoints before porting

## Probe (verify sync before writing Rust)

```sh
cd probe
uv run login.py us        # one-time; opens Amazon sign-in, paste redirect URL back
uv run probe.py           # lists library + last positions
uv run probe.py ASIN MS   # writes a position — check the phone app picks it up
```

## Running

```sh
cargo run                 # GUI; first launch shows the Amazon sign-in page
cargo run -- open ASIN    # GUI, jump straight into a book (dev)
cargo run -- login [locale] / probe [ASIN [MS]] / chapters ASIN / download ASIN / play ASIN [SECS]
```

Auth lives in `~/.config/skald/auth.json`; decrypted books and covers in `~/.cache/skald/`.
Needs `gst-libav` (AAC decoder) and `ffmpeg` (one-shot lossless AAXC → m4b remux).

## API facts verified against a live account (2026-08-27)

- Registration returns a **PKCS#1** (`BEGIN RSA PRIVATE KEY`) device key.
- `GET /1.0/library` items may have explicit `null` for `authors`/`narrators`.
- `GET /1.0/annotations/lastpositions?asins=…` — **max 25 ASINs per call**; response is
  `{asin_last_position_heard_annots:[{asin, last_position_heard:{position_ms,last_updated,status}}]}`.
- `POST /1.0/content/{asin}/licenserequest` → `content_license{acr, status_code:"Granted", license_response, content_metadata{content_url.offline_url, last_position_heard, chapter_info, content_reference}}`.
  Voucher: AES-128-CBC, key‖iv = SHA256(device_type+device_serial+customer user_id+asin); decrypts to `{key, iv}`.
- `PUT /1.0/lastpositions/{asin}` body `{acr, asin, position_ms}` → empty 2xx body. **Confirmed to
  propagate to the phone app**, and the phone's own position comes back via the same read endpoint.
- Sync policy: never push before the resume-seek has landed (an early push clobbers the phone's
  position); push every 30 s while playing and on pause/seek/close.

## Scope

Skald is an interoperability client for **your own** Audible account: it signs in as a
device, streams the books that account already owns, and reads and writes the same
last-position annotations the official apps do. It is not a downloader for content you
have not bought, and it strips nothing that Audible's own apps do not.

Auth lives at `~/.config/skald/auth.json`, mode 0600 — it holds the device private key
and the refresh token, so treat it exactly like a password.

## Licence

MIT — see [LICENSE](LICENSE).
