# Skald

*Skald* (Old Norse *skáld*): a poet and storyteller of the Viking-age Norse courts,
who composed and recited verse from memory. A fitting name for an app whose job is
to read you stories.

Skald is a desktop player for your own Audible library. It shows your books as a
wall of covers, plays them with native controls and media keys, and follows the
Omarchy theme.

## How it works

Skald does not download, decrypt, or re-implement anything. Playback is Audible's
own web player, running unseen inside the app in an embedded Chromium (Qt
WebEngine). Skald reads what that page shows and presses its controls. Licensing,
streaming, and position sync between devices are therefore all Audible's, exactly
as in a browser tab.

- You sign in once, on Amazon's real sign-in page, shown in the window.
- The library is read from the same library pages the site shows you.
- `W` shows the underlying Audible player whenever you want to see it.

Because it reads Audible's pages, a redesign on their side can break the library
or the controls until Skald is updated. When a book does not start, Skald says so
and offers the page itself.

## Requirements

Audible streams with Widevine. Qt WebEngine has no Widevine of its own and borrows
the module from a Chromium or Chrome profile, so one of those must be installed and
have played something protected once (which is when it downloads the module). Skald
tells you at startup if it cannot find it.

## Shortcuts

| Key | Action |
| --- | --- |
| `Space` | Play or pause |
| `←` `→` | Back or forward 30 seconds |
| `Shift` + `←` `→` | Previous or next chapter |
| `-` `=` | Slower or faster |
| `C` | Show or hide the chapters |
| `/` | Search the library |
| `Enter` | Open the selected book |
| `P` | Go to the book that is playing |
| `Esc` | Back to the library |
| `R` | Read the library from Audible again |
| `W` | Show or hide Audible's own player |
| `?` | Show the shortcuts |
| `Q` | Quit |

Media keys work too: play/pause, and next/previous move by chapter.

## Building

```sh
bin/build      # qmake6 + make into build/skald
bin/test       # backend tests, offscreen
bin/install    # builds the package and installs it, so Skald is in the launcher
```

## Files

- `~/.local/share/skald/QtWebEngine/` holds the browser profile, including the
  Audible sign-in cookies. Treat it like a password. "Sign out" on the `?` screen
  clears it.
- `~/.cache/skald/` holds the last library listing and the cover images.
- `~/.config/skald/skald.ini` holds the playback speed and the last book. Set
  `domain=www.audible.co.uk` (or another marketplace) there to use a different
  Audible site.

## Layout

- `src/backend.*`: state: the library, and what the player page reports
- `src/library.*`: the list of titles and its search
- `src/mpris.*`: media keys over D-Bus
- `src/web/*.js`: the two small scripts that run inside Audible's pages
- `src/*.qml`: the window

## Licence

MIT, see [LICENSE](LICENSE).
