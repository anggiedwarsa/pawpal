<p align="center"><img src="assets/icon.png" alt="PawPal icon" width="128"></p>

# 🐾 PawPal

A free, realistic desktop pet that lives on your **Dock** (macOS) or **taskbar** (Windows, Linux).
Import short clips of your own cat. PawPal cuts the cat out of the background, and it walks, naps,
chases balls and butterflies, eats from its bowl and keeps you company while you work.

## Features

- **Your real pet**: import a video, GIF, WebP or APNG per action (walk, sit, sleep, eat…). The background is removed automatically.
- **Free will**: pets pick what to do from their hunger, energy and mood, and from the clips they have.
- **Lives anywhere on screen**: drag a pet up the screen and it makes a new home there. Food, balls and butterflies appear on the level the pet is on.
- **A daily rhythm**: pets stretch and groom in the morning, play more in the afternoon and calm down in the evening. At bedtime (10 PM by default; 9, 11, midnight or off under ⚙️ Settings → 🌙 Bedtime) they say goodnight, go to bed in the corner and sleep through until 7, then wake up with a "Good morning! ☀️".
- **Tricks**: right-click a pet → 🎾 Tricks to ask for Sit, High Five, Speak, Bow, Jump or Dance. It's proud of itself afterwards (⭐, a little happier), but a grumpy or sleepy pet may not feel like it.
- **Perches on your windows**: now and then a pet hops up onto the top edge of the window you're using and strolls or naps along it. Move the window and it rides along; close it, resize it or switch to another window and it jumps back down. Turn it off with 🪟 Perch on Windows under ⚙️ Settings.
- **Bedtime in the corner**: a sleepy pet (or one sent to sleep) hops down level by level, walks to the end of the Dock and hops down into the bottom-right corner of the screen to nap. Click it and it mumbles "What's up? I'm so sleepy… 😴"; double-click wakes it for 30 seconds before it goes back to bed.
- **Movie mode**: when a video or app goes full screen, pets duck down and peek from the bottom-right corner with just the top of their face showing, then come back out afterwards.
- **Calendar heads-ups**: 10, 5 and 1 minute before an event, a pet walks out, jumps to the middle of the screen and tells you what's coming up for 30 seconds. Click it (or wait) and it jumps back to the same spot and height and carries on with what it was doing. Turn it off under ⚙️ Settings.
- **Low battery heads-ups**: on a laptop running on battery, a pet comes out at 10% and again at 5% to ask you to plug in.
- **Your own reminders**: tray menu → ⏰ Remind Me…, type what and when ("20 min", "1h 30m", "3:30 pm", up to a day ahead), and a pet comes out to tell you, the same way as for a calendar event. They survive a restart; cancel one from ⏰ Upcoming Reminders. Works on every platform.
- **Break nudges**: after 50 minutes at the computer (or 25 or 90, or off, under ⚙️ Settings → 🧘 Break Reminders), a pet comes to the middle of the screen and suggests a stretch, some water or a breath away from the screen. Five minutes without touching the keyboard or mouse counts as a break, and nudges wait while something plays full screen.
- **Focus mode**: tray menu → 🎯 Focus for 25, 50 or 90 minutes. The pets curl up in bed and leave you be (no break nudges), then wake and one comes to say well done. Calendar heads-ups and your own reminders still come through.
- **Quiet during calls**: when a call starts, pets go to bed on their own; when it ends they wake up by themselves (it's checked every few seconds, and the call counts as over after 45 seconds with no sign of it). A call is the mic or camera being on (any app: Meet, Zoom, Teams…) or a Google Meet call open in a browser, which also catches Meet while you're muted. If you'd rather not have this at all, untick 📞 Focus During Calls under ⚙️ Settings.
- **Dances to music**: while something is playing sound, pets often break into a dance. Turn it off with 🎵 Dance to Music under ⚙️ Settings.
- **Out of sight while you share your screen**: when a browser shows its "… is sharing your screen" bar (Meet, or any site), or Zoom's share toolbar is up, pets duck all the way into the corner and come back out when you stop. During calls the window is also kept out of screen captures where the OS allows it, so viewers don't see the pets even for a moment.
- **Welcome back**: come back after five minutes or more away from the keyboard and mouse and the nearest pet waves hello. After an hour or more it missed you and does a happy dance.
- **Seasonal touches**: every few minutes a little flurry drifts down over the Dock: snow in December and January, leaves in October and November, blossoms in April, bats and pumpkins on Halloween and confetti on New Year's Eve (seasons follow the northern hemisphere). Turn it off with ✨ Seasonal Touches under ⚙️ Settings.
- **Keeps out of your way**: while you type, pets sit and watch, and move away from the text you're typing (exactly where the OS reports the caret, otherwise around the front window).
- **Click-through**: clicks pass to the apps underneath unless the cursor is over a pet or a toy.
- **Tray menu**: feed, give treats, throw a ball, release a butterfly, pick activities, set reminders and focus. Everything you set once (size, walk area, bedtime, break reminders, launch at login…) is together under ⚙️ Settings.

## Platform support

| | macOS | Windows | Linux (X11 / XWayland) |
|---|---|---|---|
| Stands on | the Dock (exact icon strip with Accessibility) | the taskbar | the bottom panel (`_NET_WORKAREA`) |
| Avoids typing | exact caret (Accessibility) | exact caret in Win32 apps, else front window | front window |
| Right-click pet menu | ✅ | ✅ | via the tray menu |
| Background removal | edge flood-fill | edge flood-fill | edge flood-fill |
| Calendar heads-ups | Calendar app accounts (asks for access once) | — | — |
| Full-screen detection | a front-app window covers the screen | foreground window covers its monitor | `_NET_WM_STATE_FULLSCREEN` |

On Wayland sessions PawPal runs through XWayland, because Wayland doesn't let apps place their windows.

## Download

Ready-to-install builds are on the [Releases page](https://github.com/anggiedwarsa/pawpal/releases/latest):

- **macOS** (Apple silicon and Intel): `PawPal-<version>-macOS.dmg`. Drag PawPal into Applications. The app isn't notarized yet, so the first time right-click it → **Open**.
- **Windows** (10/11, 64-bit): `PawPal-<version>-Windows-Setup.exe`. Installs for your user, no admin needed.
- **Linux**: `pawpal_<version>_amd64.deb` (`sudo apt install ./pawpal_<version>_amd64.deb`), or the `.tar.gz` for other distributions.

Releases are built by GitHub Actions whenever a `v*` tag is pushed (`.github/workflows/release.yml`).

## Build from source

You need a recent stable [Rust](https://rustup.rs).

**macOS**: builds `PawPal.app` with its icon. `--install` copies it to `/Applications`; `--universal` builds for both Apple silicon and Intel.

```sh
./build.sh --install
```

**Windows / Linux**

```sh
cargo build --release
./target/release/pawpal
```

Runtime extras:

- **ffmpeg** on your `PATH` to import video files (GIF, WebP and APNG work without it).
- **Linux**: a StatusNotifier tray (KDE and most panels; GNOME needs the AppIndicator extension), plus `zenity` or `kdialog` for the name and rename prompts.

## Usage

Click 🐾 in the menu bar or tray. Choose **➕ Adopt / Import a Pet → Import Clip**, pick the action, choose a clip, and name your pet.
Use the same name for every clip of the same cat. For best results, film the same cat from the side against a plain background for 2–6 seconds.

- **Click** a pet to pet it. **Double-click** to make it jump.
- **Drag** a pet anywhere: dropped up on the screen, it lives there; dropped near the Dock, it lands on it.
- **Right-click** a pet for its menu (stats, activities, rename, add clips).

Command line:

```sh
pawpal --import clip.mp4 "My Cat" walk
pawpal --import-sheet sheet.png "My Cat" "walk=9@10/30,sit=0@5" ["Art by …"]
```

Clips are stored in `<data dir>/PawPal/Skins/<pet>/<action>/` (on macOS, `~/Library/Application Support/PawPal/Skins`).
Settings are in `<config dir>/PawPal/settings.json`. On macOS, settings from the old Swift version are carried over automatically.

## Architecture

Layers depend only downward. The engine knows nothing about drawing or the OS, and only `platform/` contains OS-specific code.

```
src/
├── main.rs              CLI entry (imports) and app start
├── app/                 window, frame loop, tray and pet menus, mouse input, import flow
├── render/              OpenGL drawing (femtovg) and text/emoji (cosmic-text)
├── engine/              the simulation: world, pet behavior, toy physics
├── domain/              pets, activities, species, props (plain data)
├── data/                settings, clip library, clip and sprite-sheet importers
├── platform/            per-OS code behind one API: macos.rs, windows.rs, linux.rs
└── support.rs           geometry, colour and random helpers
```

Each source file has its unit tests in a sibling `*_tests.rs` file (e.g. `engine/world.rs` → `engine/world_tests.rs`).

## Development

```sh
cargo test                                         # unit tests
cargo check --target x86_64-pc-windows-msvc        # type-check Windows from any OS
cargo check --target x86_64-unknown-linux-gnu      # type-check Linux from any OS
```
