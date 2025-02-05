# frosk
![frosk](screenshot.png)

This is a toy project which contains a prototype for a World of Warcraft fishing bot with a unique approach. It works by capturing the audio directly from the game and performing realtime digital signal processing to detect the sound of a fish biting. The bot then automatically reels in the fish and casts the line again.

More specifically, it works by maintaining a buffer of the most recently captured game audio. It then cross-correlates this buffer with a target signal (static sound of a fish biting). If the correlation is sufficiently high, the bot will simulate a key press to reel in the fish.

Todo list:
- [ ] Improve detection technique - it's probably easy to come up with an alternative approach which is both more efficient and less sensitive than the current cross-correlation implementation
- [ ] The re-casting mechanism currently just spams the key a fixed amount of time and assumes the cast will have succeeded. Maybe listen for a "cast successful" sound instead? Or do something based on the game visuals?
- [ ] Add a GUI for configuring stuff like which hotkeys to use, etc.

## Pre-requisites
It requires you to have specific settings configured in the game:
* Interact with target: `F9`
* Hotkey for fishing: `F10`

## Usage
```bash
cargo run -r --bin gui
```

## Tests
```bash
cargo test
```

## Benchmarks
```bash
cargo bench
```
