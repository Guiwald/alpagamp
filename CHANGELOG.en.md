# Changelog (English)

All notable changes to Alpagamp, in English. The French [CHANGELOG.md](CHANGELOG.md) is the reference; this file
follows it section by section, with the same version headings, so that release notes can quote both. Format
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), [SemVer](https://semver.org/) numbering adapted to the
project (see `docs/versions.md`). Each entry starts with the part of the app it concerns ("Audio: …").

## [Unreleased]

## [0.6.2] - 2026-10-15

> Rollout: Mac (`Alpagamp-0.6.2.dmg`) — replace the app in Applications. Windows — replace the folder. Android
> (`Alpagamp-0.6.2.apk`) — install over 0.6.1. Nothing to redo for Spotify: you stay signed in; the Library fills
> up the first time you open it, then shows up instantly.

### Added
- Spotify: Library lists (playlists, liked songs, albums, the content of a playlist or an album) are kept on disk:
  they show up at once, even offline, and are only fetched from Spotify again when they are more than 15 minutes
  old, or with the **Refresh** line at the top of the list (which shows the list's age).
- Spotify: when Spotify asks you to wait, the Library says so with a **countdown** ("try again in 11:42") above
  the last list received, and asks for nothing before that time; when it runs out, the line becomes "Try again
  now".
- Spotify: while signing in, the Library (and the Spotify menu) offer **Reopen the sign-in page** and **Cancel**;
  clicking "Log in" again reopens the same page.
- Spotify: `alpagamp --diagnostic-spotify [report.txt]` — a handful of read-only requests on each channel, with
  response codes, waits and timings (never a token).
- Library: long lists appear as they load ("Loading… 500/2,345").

### Changed
- Spotify: the Library now goes through Spotify's internal channels (the ones its own apps use, through the
  librespot session) instead of the public web API: playlists, liked songs, albums, playlist and album contents,
  song search. The web API is only used to search for albums and playlists (ADR 0022).
- Spotify: up to 10,000 items per list (2,000 before); playlist folders are flattened, with the folder name as a
  subtitle.

### Fixed
- Spotify: "Spotify asks you to wait (too many requests)" right after signing in, with empty tabs. Spotify throttles
  its web API for tokens obtained with the desktop player's client ID, which every librespot-based player shares;
  on top of that, Alpagamp ignored `Retry-After` and asked for everything again on every tab change.
- Spotify: closing the browser tab before authorizing blocked sign-in for 5 minutes, with no way to start over.
  After 5 minutes without an answer, you now get back to "Log in", without an error message.
- Audio: music skipped (1–2 s gaps every 7–8 s) when the file was on a slow drive, such as an external disk waking
  up or being indexed by Spotlight. A thread now reads the file ahead (up to 16 MB, usually the whole song); the
  decoder no longer touches the disk.
- Interface: "File info" on a song other than the current one no longer freezes the player while the file is read.
- Release: `construire-mac.sh` and `publier-sur-github.sh` update the code even if a test run left a modified copy
  (for example `Cargo.lock`).

## [0.6.1] - 2026-10-14

> Rollout: Mac (`Alpagamp-0.6.1.dmg`) — replace the app in Applications; on first launch, the windows snap back to
> the main window at the new automatic size (×1.5 on a laptop Retina screen). Windows — replace the folder.
> Android (`Alpagamp-0.6.1.apk`) — install over 0.6.0 (nothing changes on the phone).

### Added
- Interface: half steps in Options › Window size — ×1, ×1.5, ×2, ×2.5, ×3 and automatic. On a Retina screen, ×1.5
  stays sharp (×3 real pixels); elsewhere, pixels are slightly uneven.

### Changed
- Interface: the automatic size gives ×1.5 on Retina screens 1,400 to 2,100 points wide (Mac laptops), instead of
  ×2. Cmd+D (Ctrl+D) is still Winamp's double size: ×1 ↔ ×2, back to ×1 from any other size; the "Double size"
  check mark and the D button light up as soon as the windows are enlarged.
- Docs: ADR 0021 (windows on macOS: real position, snapping back at startup, bringing to front, half steps).

### Fixed
- Mac: at startup, windows no longer overlap or come apart — each window was created small, then grew upwards
  (macOS keeps the bottom-left corner). They are now created at their size, and the group is snapped back the way
  it was when closed, even if the size changed.
- Mac: dragging the main window to the very top of the screen no longer scatters the other windows: the group stops
  under the menu bar, and follows the real position if macOS corrects one. Snapping uses the usable area of the
  screen (below the menu bar, above the Dock).
- Mac: the equalizer, the playlist and Alpagamp's other windows come to the front with the window you activate, at
  launch as on click.
- Release: the "Mac" note on release pages now says that GitHub replaces the first .dmg (Apple silicon) with a
  universal .dmg, instead of announcing Intel Macs "for the next versions".

## [0.6.0] - 2026-10-14

> Rollout: Mac (`Alpagamp-0.6.0.dmg`) — replace the app in Applications. Windows — replace the folder. Android
> (`Alpagamp-0.6.0.apk`) — install over 0.5. Plugins start switched off: right-click › Plugins (on the phone:
> menu › Plugins).

### Added
- Plugins: WebAssembly plugin system (API v1, ADR 0020) — each plugin runs in a sandbox (wasmi: no disk, no
  network, capped memory, limited computing time); a plugin that crashes or loops is stopped, switched off and
  reported in the scrolling title, and the music keeps playing.
- Plugins: **Plugins** menu (right-click, Mac menu bar, phone menu) — enable, each plugin's settings, About,
  Install a plugin… (or drop a `.wasm` onto Alpagamp), Open the folder, Reload.
- Plugins: three plugins included — *Fullscreen Bars* (retro LED spectrum analyzer in the skin's colors, fire, ice
  or rainbow, in the visualization window and full screen), *Enhancer* (wider stereo, bass, clarity, limiter),
  *Sleep Timer* (5 minutes to 1½ hours, fade, stop or pause, reminders).
- Plugins: on the phone, a plugin visualization is shown full screen, even without a graphics card.
- Development: `alpagamp-plugin` kit and examples in `plugin-examples/`, `cargo xtask plugins`.

### Fixed
- Release: the Windows package built by GitHub failed (`cargo xtask` didn't compile on Windows); continuous
  integration now checks the tooling on Mac and Windows.

## [0.5.0] - 2026-10-14

> Rollout: Mac (`Alpagamp-0.5.0.dmg`, Apple silicon and Intel) — replace the app in Applications; window positions
> may go back to their defaults once. Windows — replace the folder. Android (`Alpagamp-0.5.0.apk`) — install over
> 0.4. Visualization: Ctrl+Shift+K (Cmd+Shift+K), then right-click › Presets › Download the pack; on the phone:
> menu › Full-screen visualization. Skins: Alt+S. Size: Options › Window size.

### Added
- Visualization: visualization window (Ctrl+Shift+K, double-click on the visualizer, Visualization menu) showing
  MilkDrop presets with the lactea engine; resizable skin frame, full screen (double-click, Enter, F11; Esc to
  leave), MilkDrop keys (Space: next with blend, H: hard cut, Backspace: previous, R: random order, K or Scroll
  Lock: lock, L: pick a preset), automatic change (10 s to 2 min) and adjustable blend time, preset name shown at
  each change.
- Visualization: six original presets built in; "Download the preset pack" installs *Cream of the Crop* (9,795
  presets, about 30 MB) on demand; personal presets in the presets folder.
- Visualization: "Choose a preset" (Alt+K, L) — search the list, show one with a double-click.
- Skins: skin browser (Alt+S, Skins menu) — Museum tab (Winamp Skin Museum: featured skins, search, previews, one
  click downloads and applies) and My skins tab (previews drawn by Alpagamp); also on the phone (menu › Skin
  browser), where the chosen skin is kept.
- Skins: window shapes (`region.txt`) on Windows and macOS; skin mouse cursors (`*.cur`).
- Interface: Options › Window size (×1, ×2, ×3, automatic for the screen) for all windows; Ctrl+D (Cmd+D) switches
  from ×1 to ×2.
- Phone: full-screen visualization (menu › Visualization, or double-tap the visualizer): swipe to change preset,
  double-tap to leave, long press for its menu, Back button.
- Phone: the screen is composed by the graphics card (Vulkan), falling back automatically to software display;
  "Display through the graphics card" option to turn it off.
- Audio: continuous stream of played samples for the visualizations.
- Network: file downloads with progress.
- Docs: ADR 0019 (lactea chosen after an audit; test on 200 presets); README for everyone; publishing on GitHub
  (script, continuous integration).

### Fixed
- macOS: snapped windows stay snapped when moved from one screen to another (screens of different densities) —
  positions kept in points.
- macOS: windows too small on large screens (automatic size for the screen).
- Generic windows: titles with accents are shown (unaccented letters of `gen.bmp`).

## [0.4.0] - 2026-10-13

> Rollout: Windows (`Alpagamp-0.4.0-windows-x64.zip`) — replace the old folder. Android (`Alpagamp-0.4.0.apk`) —
> install over 0.3. For Spotify: Library (Alt+L, or menu › Library on the phone) › "Log in to Spotify…", authorize
> in the browser, come back to Alpagamp. Spotify Premium required.

### Added
- Spotify: one-click sign-in (browser, no setup), kept from one session to the next; Spotify songs play through
  Alpagamp's audio chain (equalizer, visualizer, crossfade), 320 kbit/s; `open.spotify.com` links and `spotify:`
  URIs in the playlist (playlist and album links add their songs). Spotify Premium required.
- Spotify Connect: Alpagamp shows up as a device in the Spotify app ("Alpagamp (Mac)", "Alpagamp (Android)"…);
  what you start there plays through Alpagamp, and the transport buttons control Spotify.
- Interface: Library (Alt+L) — Playlists (with Liked songs), Liked, Albums, Search and Radios tabs (radio-browser.info
  directory, search by name or genre); double-click to open or play, Play and Add buttons. On the phone, in the
  menu.
- Docs: ADR 0018 (Spotify in the engine, librespot's TLS).

## [0.3.0] - 2026-10-13

> Rollout: Windows (`Alpagamp-0.3.0-windows-x64.zip`) — replace the old folder, settings kept. Android
> (`Alpagamp-0.3.0.apk`) — open the file on the phone, allow installing from this source, install, then allow
> music access on first launch. Later Android versions will install over it (same signing key).

### Added
- Audio: internet radio (SHOUTcast, Icecast; HTTP and HTTPS; MP3, AAC, OGG) with the current song in the scrolling
  title and the playlist, the station name in the file info; `.pls` / `.m3u` radio playlists followed; buffering
  without blocking the interface. Add one with "Add URL" (Ctrl+L), by dropping a link, or in a playlist.
- Audio: adjustable crossfade (1 to 10 s) between songs, at constant power.
- Audio: ReplayGain (track or album), preamp, clipping prevention.
- Playback: bookmarks — add the current song (Ctrl+Alt+B), play them again from the Bookmarks menu, remove them.
- Interface: every visualizer option — analyzer style (normal, fire, line), thick or thin bars, peaks, falloff
  speeds, oscilloscope style (dots, lines, solid), refresh rate.
- Interface: file info with the station and ReplayGain values.
- Android: first version (`Alpagamp-0.3.0.apk`) — main window, equalizer and playlist stacked to the screen width,
  for touch (long press for the menu, double-tap to play, swipe to scroll the playlist); the phone's music
  (permission asked on first launch); ready-to-play radio stations and addresses to type; the same settings as on
  the desktop (equalizer, crossfade, ReplayGain, visualizer, languages).
- Tools: `cargo xtask apk` (signed APK without Gradle) and a phone simulator on the computer.
- Docs: ADR 0017 (Android without Java).

### Changed
- Network: the system's root certificates are used (corporate proxies), the `HTTPS_PROXY` proxy is respected;
  encryption by graviola (Rust, no C).

## [0.2.0] - 2026-10-13

> Rollout: Windows (`Alpagamp-0.2.0-windows-x64.zip`) — close Alpagamp, replace the old folder with the new one;
> settings, playlist and skins are kept. The equalizer and the playlist open under the main window (Alt+G, Alt+E
> to hide them). The Mac version comes with continuous integration.

### Added
- Interface: 10-band equalizer and preamp — sliders (snapping at 0 dB, Shift for fine control), click +12 / 0 / −12
  to set every band, response curve, ON and AUTO buttons, values in the scrolling title, windowshade mode with
  volume and balance.
- Equalizer: factory presets, opening and saving `.eqf` files, automatic preset per song (AUTO button), reset to
  flat.
- Interface: resizable playlist (25×29 steps) — titles and durations read in the background, click to select
  (Shift, Ctrl), drag to reorder, mouse wheel and scroll bar, ADD / REM / SEL / MISC / LIST menus, mini transport,
  duration of the selection and of the list, windowshade, right-click menu.
- Interface: windows snap to each other and to the screen edges (Shift not to snap); the main window drags the
  snapped windows along; windowshade, double size and resizing shift the stacked windows.
- Interface: "Jump to file" (J) and "Add URL" (Ctrl+L) in skinned windows.
- Interface: Alt+G (equalizer), Alt+E (playlist), J shortcuts; in the playlist: arrows, PgUp / PgDn, Home / End,
  Enter, Delete, Ctrl+A, Ctrl+I, Alt+↑ / ↓, Ctrl+N / O / S.
- Playlist: save (M3U8, M3U, PLS) and open a playlist, sort (title, file name, path), reverse, shuffle, remove
  missing files or duplicates.
- Skins: colors and letters of the generic windows (`genex.bmp`, `gen.bmp`); sharp playlist text on Retina screens
  (Liberation Sans font).
- Docs: README, ADR 0016 (windows).

### Changed
- Interface: on Windows, a single taskbar entry; the secondary windows follow the main one.
- Menus: Equalizer and Playlist editor entries; macOS menu bar with File and Windows.

### Fixed
- Interface: the app redrew its windows non-stop, which kept the CPU busy (especially on Windows).
- Skins: the title letters of the default skin's generic windows were unreadable.

## [0.1.0] - 2026-10-13

> Rollout: first preview, for Windows only (`Alpagamp-0.1.0-windows-x64.zip`): unzip, run `alpagamp.exe`; on
> first launch, "More info" then "Run anyway". The Mac and Android versions come next. No Equalizer and Playlist
> windows yet (0.2).

### Added
- Interface: complete main window, pixel for pixel — title bar, clutterbar (O A I D V), elapsed or remaining time
  (click), scrolling title, bitrate and sample rate, mono/stereo, volume and balance with status messages, seek
  bar, transport, shuffle, repeat, windowshade mode (double-click the title), double size, always on top.
- Interface: built-in visualizer — spectrum analyzer (19 thick or 75 thin bars, falloff and peaks) and
  oscilloscope; a click cycles Analyzer → Oscilloscope → Off.
- Skins: classic `.wsz` skins (drop a file onto the window, or the Skins menu), file-by-file fallback to the default
  skin, tolerant BMP decoder (RLE4/RLE8), `viscolor.txt`, `pledit.txt`, `region.txt`.
- Skins: original default skin (graphite metal, mint LCD display, alpaca).
- Audio: local files MP3, FLAC, OGG Vorbis, WAV, AIFF, AAC/M4A, ALAC; gapless playback between songs; resampling to
  the output; ReplayGain ready; fade-out stop (Shift + Stop) and stop after the current song (Ctrl + Stop).
- Playback: internal playlist (folders scanned, M3U/M3U8/PLS), shuffle without repeats, repeat, manual advance;
  the playlist and settings are kept from one session to the next.
- Interface: classic shortcuts (Z X C V B, arrows, L, Shift+L, R, S, Ctrl+D, Ctrl+T…), mouse wheel for volume,
  full right-click menu, file info.
- Interface: menus in English, French, Spanish, German, Italian, Valencian and Irish (system language).
- Windows: standalone executable with icon.
- Docs: Winamp technical reference, architecture decisions (ADR 0001 to 0015), glossary.
