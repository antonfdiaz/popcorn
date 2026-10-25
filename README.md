# Popcorn

Popcorn is a cross-platform 2D animation editor built with Rust and GPUI Kit. It brings a classic timeline workflow to vector animation, with drawing tools, layers, keyframes, and `.pop` project files.

**Note**: This project was highly AI assisted, but there were some human edits too. If you don't like that, I completely understand it.

## Features

- Draw with selection, pencil, shape, text, fill, and eraser tools.
- Animate with layers, frames, keyframes, playback, and looping.
- Select and transform objects or groups; move and edit timeline ranges.
- Import images, edit text, and undo or redo changes.
- Adjust fills and object properties in the inspector.

## Run

Install [Rust](https://www.rust-lang.org/tools/install), then run:

```sh
cargo run
```

Run the test suite with `cargo test`.

## Shortcuts

| Shortcut | Action |
| --- | --- |
| `Enter` | Play or pause |
| `F5` / `F6` / `F7` | Extend timeline / insert keyframe / insert blank frame |
| `Shift` + `F5` | Delete selected timeline frames |
| `Home` / `End` | Go to first / last frame |
| `V` / `Y` / `K` / `E` | Select / pencil / fill / eraser |
| `R` / `O` / `T` / `H` / `Z` | Rectangle / ellipse / text / hand / zoom |
| `Shift` + click | Select a range of frames or layers |
| `Delete` | Remove selected timeline contents |
| `Cmd/Ctrl` + `Z` | Undo |
| `Cmd/Ctrl` + `Shift` + `Z` | Redo |

## Project layout

Editor code is organized by responsibility under `src/editor/`, including the canvas, timeline, commands, panels, and UI.
