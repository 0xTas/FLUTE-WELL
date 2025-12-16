<div align="center">
<h1>FLUTE WELL</h1>
</div>

<p align="center">
Play MIDI files on the Animal Well flute using automated keyboard input.
</p>

---

**FLUTE WELL** is a command-line tool that takes a standard **MIDI file** and plays it on the in-game flute from **ANIMAL WELL** by synthesizing keyboard input at precise timings.<br>
<br>
It does **not** modify the game, inject code, or emulate audio.<br>  
It simply reads a MIDI file, converts the notes to flute fingerings, and sends the corresponding key presses to the operating system.<br>
<br>
I spent most of my playthrough thinking you could only play 8 notes of the A Major scale on the flute, and didn't discover that you could play semitones and drop the octave until after I had beaten the game.<br>
As soon as I found that out, I felt compelled to make this.<br>
<br>
Afaik, the range of the Animal Well flute is **A4 through A6** (*midi 69..=93*), which is still a bit limited compared to a real flautist's range, but definitely serviceable.<br>
<br>
FLUTE WELL does best with single-track midi files containing pitches that fall exclusively within that range, but will automatically attempt to transpose errant notes by octave until they fit the range, and resolves polyphonic midi events down to monophonic melodies as per a configurable policy.<br>
<br>
To see FLUTE WELL in action, [**click here**](https://www.youtube.com/watch?v=uAFeHlPxMU8).

---

## Features

- 🎵 Reads standard `.mid` files directly.
- ⏱ High-precision timing (sub-millisecond scheduling).
- 🎹 Single-note melody extraction (polyphony reduction).
- 🎼 Supports octave & semitone transposition.
- 🎶 Supports articulation presets & custom hold-percentages.
- 🧪 Dry-run mode for inspecting & debugging imported events.

---

## Non-Goals

FLUTE WELL intentionally does **not** attempt to:
- Support chords (the flute is monophonic).
- Preserve MIDI instruments or layers.
- Provide a visual editor.
- Run inside the game.
- Bypass OS or game input restrictions.

If a MIDI file contains chords, a **single note is selected per moment** using a configurable policy (e.g. highest note).

---

## Requirements

### Game
- **ANIMAL WELL**
- The flute must be unlocked and equipped (not just selected..!)
- The game window must be focused during playback, and its title cannot be altered

### Platform

> [!IMPORTANT]
> Currently, the program only runs on Windows, as a Linux-compatible InputEngine hasn't yet been implemented.<br>
> Animal Well does work on Linux via Proton, so I may implement this at some point in the future.<br>
> I'm also open to accepting PRs if you would like to implement it yourself!

---

## Usage
1. Download the latest release or compile the program from source.
2. Run ANIMAL WELL, and equip your flute by pressing E (or controller equivalent).
3. In a terminal, run `./FLUTE_WELL.exe --help` to see all available configuration flags.
4. Once you have sourced a `.mid` file, run `./FLUTE_WELL.exe [OPTIONS] <./path/to/midi/file.mid>`.
5. Tab into ANIMAL WELL and allow it to remain as the focused window for the duration of the chosen song's playback.

### Examples
```
./FLUTE_WELL.exe --articulation tenuto --verbose --delay-start 5 ./badinerie_js_bach.mid

./FLUTE_WELL.exe --articulation custom --hold-percentage 0.69 --transpose 2 -v ./twinkle_twinkle_little_star.mid

./FLUTE_WELL.exe -a s -t 5 -v --dry-run ./the_flight_of_the_bumblebee.mid
```

>[!TIP]
> FLUTE WELL uses Rust's `env_logger` crate to output information to the terminal.<br>
> By default you won't see much, so you should set your `RUST_LOG` environment variable to "info" in order to see any runtime information.<br>
> You can then pass the `--verbose` command-line arg to the program in order to see additional information output, like individual notes and their durations.<br>
> You may instead wish to set the log level to "debug" if you are contributing to the repo or debugging an encountered issue.

---

## Building & Contributing

FLUTE WELL is written in Rust.<br>
To compile it you'll need [Rust & Cargo](https://rustup.rs) installed.<br>
Then, run `cargo build --release` and use the created binary in the `./target/release` directory.<br>
<br>
> [!IMPORTANT]
> When running `cargo test`, the `-- --test-threads 1` flag should be passed to prevent multiple tests from attempting to play the flute at the same time.<br>
> You can also set your `RUST_LOG` environment variable to "info" or "debug", and then use the flag `--nocapture` in order to see potentially helpful information in the terminal.<br>
> Example: `cargo test -- --test-threads 1 --nocapture`.
