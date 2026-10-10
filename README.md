# Popcorn

Popcorn is a cross-platform 2D animation editor inspired by Adobe Flash/Animate, built with Rust and GPUI Kit.

The goal is to provide a fast, extensible, classic timeline workflow without scripting or advanced publishing features for now.

## Current status

- Canvas with selection, move, hand, zoom, rectangle, ellipse, pencil, and text tools.
- Timeline with layers, frames, keyframes, blank frames, playback, and looping.
- Shift-click selection for multiple frames and layers, with block movement, reordering, and deletion.
- Drag selected keyframes horizontally to move them together in the timeline.
- Selection controls for scaling and rotating objects.
- Multiple selection with group move, scale, and rotation controls.
- Image import through drag and drop, as well as copy and paste.
- Double-click text to edit it and `Escape` to cancel the current operation.
- Inspector with properties, object library, and alignment tools.
- Fill colors with quick swatches and a custom color picker.
- Context menus, undo/redo, and `.pop` documents.
- Dark interface with native macOS integration.

## Running

You need Rust and Cargo installed:

```bash
cargo run
```

To check the project:

```bash
cargo test
```

## Main shortcuts

- `Enter`: play or pause the animation.
- `F5`: extend the timeline.
- `F6`: insert a keyframe.
- `F7`: insert a blank frame.
- `Shift`: constrain shapes and transformations.
- `Shift` + click: select a range of timeline frames or layers.
- `Delete` on selected timeline frames: remove their keyframes and contents.
- `Cmd/Ctrl + Z`: undo.
- `Cmd/Ctrl + Shift + Z`: redo.

## Structure

The logic is split by responsibility inside `src/editor/`: canvas and tools, timeline, commands, panels, and UI. The document format and animation models are designed to support more tools and publishing capabilities over time.
