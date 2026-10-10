use super::*;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use std::path::PathBuf;
use workspace::Object;

#[derive(Clone, Copy, Debug)]
pub(super) enum Command {
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Duplicate,
    Delete,
    New,
    Open,
    Save,
    SaveAs,
    AddLayer,
    DeleteLayer,
    RenameLayer,
    LayerUp,
    LayerDown,
    Keyframe,
    BlankKeyframe,
    ClearKeyframe,
    Extend,
    Grid,
    Fit,
    ZoomIn,
    ZoomOut,
    Onion,
    Loop,
    Properties,
    Library,
    Align,
    DocumentSettings,
    ObjectSettings,
    Help,
    Left,
    Right,
    Top,
    Bottom,
    CenterX,
    CenterY,
    Front,
    Back,
    SaveSymbol,
    PlaceSymbol(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_editor() -> Editor {
        let mut editor = Editor::default();
        editor.drawing.objects.push(Object {
            kind: workspace::Kind::Rectangle,
            x: 300.,
            y: 130.,
            w: 140.,
            h: 170.,
            layer: 0,
            keyframe: 1,
            fill: theme::ACCENT,
            rotation: 0.,
        });
        editor
    }

    #[::core::prelude::v1::test]
    fn new_editor_starts_with_one_empty_layer() {
        let editor = Editor::default();
        assert_eq!(editor.layers.len(), 1);
        assert_eq!(editor.layers[0].name, "Layer 1");
        assert_eq!(editor.layers[0].keyframes, vec![1]);
        assert!(editor.drawing.objects.is_empty());
    }

    #[::core::prelude::v1::test]
    fn keyframes_hold_independent_copies_and_blank_frames_hide_content() {
        let mut e = seeded_editor();
        e.selected_layer = 0;
        e.current_frame = 8;
        e.apply_edit(Command::Keyframe);
        assert_eq!(e.layers[0].keyframes, vec![1, 8]);
        assert_eq!(e.drawing.objects.len(), 2);
        e.drawing.objects[1].x = 500.;
        assert_eq!(e.drawing.objects[0].x, 300.);
        assert!(!e.object_visible(&e.drawing.objects[0]));
        assert!(e.object_visible(&e.drawing.objects[1]));
        e.current_frame = 12;
        e.apply_edit(Command::BlankKeyframe);
        assert!(e.drawing.objects.iter().all(|o| !e.object_visible(o)));
        e.apply_edit(Command::ClearKeyframe);
        assert!(e.object_visible(&e.drawing.objects[1]));
    }
    #[::core::prelude::v1::test]
    fn undo_redo_restores_objects_and_invalidates_redo_after_new_edit() {
        let mut e = seeded_editor();
        e.drawing.selected = Some(0);
        e.selected_layer = 0;
        e.apply_edit(Command::Duplicate);
        assert_eq!(e.drawing.objects.len(), 2);
        e.apply_edit(Command::Undo);
        assert_eq!(e.drawing.objects.len(), 1);
        e.apply_edit(Command::Redo);
        assert_eq!(e.drawing.objects.len(), 2);
        e.apply_edit(Command::Undo);
        e.apply_edit(Command::AddLayer);
        assert!(e.future.is_empty());
    }
    #[::core::prelude::v1::test]
    fn reorder_and_delete_layers_remap_objects_and_keep_one_layer() {
        let mut e = seeded_editor();
        e.apply_edit(Command::AddLayer);
        e.selected_layer = 0;
        e.selected_layer = 0;
        e.apply_edit(Command::LayerDown);
        assert_eq!(e.drawing.objects[0].layer, 1);
        assert_eq!(e.layers[1].name, "Layer 1");
        e.selected_layer = 0;
        e.apply_edit(Command::DeleteLayer);
        assert_eq!(e.drawing.objects[0].layer, 0);
        e.apply_edit(Command::DeleteLayer);
        assert_eq!(e.drawing.objects.len(), 1);
        e.apply_edit(Command::DeleteLayer);
        e.apply_edit(Command::DeleteLayer);
        assert_eq!(e.layers.len(), 1);
    }
    #[::core::prelude::v1::test]
    fn locked_layers_reject_keyframes_and_paste() {
        let mut e = seeded_editor();
        e.selected_layer = 0;
        e.drawing.selected = Some(0);
        e.apply_edit(Command::Copy);
        e.layers[0].locked = true;
        e.current_frame = 8;
        e.apply_edit(Command::Keyframe);
        e.apply_edit(Command::Paste);
        assert_eq!(e.layers[0].keyframes, vec![1]);
        assert_eq!(e.drawing.objects.len(), 1);
    }
    #[::core::prelude::v1::test]
    fn document_round_trip_preserves_timeline_and_rejects_invalid_references() {
        let mut e = seeded_editor();
        e.selected_layer = 0;
        e.current_frame = 10;
        e.apply_edit(Command::Keyframe);
        let json = serde_json::to_string(&e.snapshot()).unwrap();
        let mut decoded: Snapshot = serde_json::from_str(&json).unwrap();
        assert!(decoded.validate().is_ok());
        assert_eq!(decoded.objects.len(), 2);
        assert_eq!(decoded.layers[0].keyframes, vec![1, 10]);
        decoded.objects[0].layer = 100;
        assert!(decoded.validate().is_err());
        let mut decoded = e.snapshot();
        decoded.layers[0].keyframes = vec![1, 10, 8];
        assert!(decoded.validate().is_err());
        let mut decoded = e.snapshot();
        decoded.fps = 0;
        assert!(decoded.validate().is_err());
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Snapshot {
    version: u32,
    layers: Vec<LayerState>,
    objects: Vec<Object>,
    symbols: Vec<Object>,
    frame_count: usize,
    fps: u32,
    stage_size: [f32; 2],
    fill: u32,
}

impl Snapshot {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported document version".into());
        }
        if self.layers.is_empty()
            || self.layers.len() > 200
            || !(1..=10000).contains(&self.frame_count)
            || !(1..=120).contains(&self.fps)
            || self
                .stage_size
                .iter()
                .any(|n| !n.is_finite() || *n < 16. || *n > 8192.)
            || self.layers.iter().any(|l| {
                l.keyframes.is_empty()
                    || l.keyframes[0] != 1
                    || l.keyframes.windows(2).any(|w| w[0] >= w[1])
                    || l.keyframes.iter().any(|f| *f > self.frame_count)
            })
            || self.objects.iter().any(|o| {
                o.layer >= self.layers.len()
                    || !self.layers[o.layer].keyframes.contains(&o.keyframe)
                    || [o.x, o.y, o.w, o.h].iter().any(|v| !v.is_finite())
                    || !o.rotation.is_finite()
                    || o.w <= 0.
                    || o.h <= 0.
            })
        {
            return Err("Invalid document dimensions, layers or frames".into());
        }
        Ok(())
    }
}

impl Editor {
    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            version: 1,
            layers: self.layers.clone(),
            objects: self.drawing.objects.clone(),
            symbols: self.symbols.clone(),
            frame_count: self.frame_count,
            fps: self.fps,
            stage_size: self.stage_size,
            fill: self.fill,
        }
    }

    pub(super) fn checkpoint(&mut self) {
        let snap = self.snapshot();
        if self
            .history
            .last()
            .is_none_or(|old| serde_json::to_string(old).ok() != serde_json::to_string(&snap).ok())
        {
            self.history.push(snap);
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
        self.future.clear();
    }

    fn restore(&mut self, snap: Snapshot) {
        self.layers = snap.layers;
        self.drawing.objects = snap.objects;
        self.symbols = snap.symbols;
        self.frame_count = snap.frame_count;
        self.fps = snap.fps;
        self.stage_size = snap.stage_size;
        self.fill = snap.fill;
        self.selected_layer = self.selected_layer.min(self.layers.len() - 1);
        self.current_frame = self.current_frame.min(self.frame_count);
        self.drawing.selected = None;
        self.drawing.drag = None;
    }

    pub(super) fn active_keyframe(&self, layer: usize) -> usize {
        self.layers[layer]
            .keyframes
            .iter()
            .copied()
            .take_while(|f| *f <= self.current_frame)
            .last()
            .unwrap_or(1)
    }

    pub(super) fn object_visible(&self, o: &Object) -> bool {
        self.layers[o.layer].visible && o.keyframe == self.active_keyframe(o.layer)
    }

    pub(super) fn editable_selection(&self) -> Option<usize> {
        self.drawing.selected.filter(|i| {
            self.drawing
                .objects
                .get(*i)
                .is_some_and(|o| self.object_visible(o) && !self.layers[o.layer].locked)
        })
    }

    pub(super) fn select_frame(&mut self, layer: usize, frame: usize) {
        self.selected_layer = layer;
        self.current_frame = frame;
        self.drawing.selected = None;
        self.drawing.drag = None;
    }

    pub(super) fn command(
        &mut self,
        command: Command,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.modal.is_some() {
            return;
        }
        if matches!(command, Command::Paste) && self.clipboard.is_none() {
            if let Some(item) = cx.read_from_clipboard() {
                if let Some(ClipboardEntry::Image(image)) = item
                    .entries()
                    .iter()
                    .find(|entry| matches!(entry, ClipboardEntry::Image(_)))
                {
                    self.clipboard = Some(Object {
                        kind: workspace::Kind::Image {
                            bytes: image.bytes.clone(),
                            format: image.format.mime_type().into(),
                        },
                        x: 100.,
                        y: 100.,
                        w: 320.,
                        h: 200.,
                        layer: self.selected_layer,
                        keyframe: self.active_keyframe(self.selected_layer),
                        fill: self.fill,
                        rotation: 0.,
                    });
                }
            }
        }
        let system_image = if matches!(command, Command::Copy) {
            self.editable_selection()
                .and_then(|index| match &self.drawing.objects[index].kind {
                    workspace::Kind::Image { bytes, format } => {
                        Some((bytes.clone(), format.clone()))
                    }
                    _ => None,
                })
        } else {
            None
        };
        match command {
            Command::New => self.modal = Some(panels::Modal::confirm_new(window, cx)),
            Command::Open => self.open_document(cx),
            Command::Save => self.save_document(false, cx),
            Command::SaveAs => self.save_document(true, cx),
            Command::RenameLayer => {
                self.modal = Some(panels::Modal::rename(
                    &self.layers[self.selected_layer].name,
                    window,
                    cx,
                ))
            }
            Command::DocumentSettings => {
                self.modal = Some(panels::Modal::document(self, window, cx))
            }
            Command::ObjectSettings => {
                if let Some(i) = self.editable_selection() {
                    self.modal = Some(panels::Modal::object(&self.drawing.objects[i], window, cx));
                }
            }
            Command::Help => self.modal = Some(panels::Modal::help()),
            _ => self.apply_edit(command),
        }
        if let Some((bytes, format)) = system_image {
            if let Some(format) = ImageFormat::from_mime_type(&format) {
                cx.write_to_clipboard(ClipboardItem::new_image(&Image::from_bytes(format, bytes)));
            }
        }
        self.status = format!("{command:?}");
        cx.notify();
    }

    pub(super) fn apply_edit(&mut self, command: Command) {
        match command {
            Command::Undo => {
                if let Some(s) = self.history.pop() {
                    self.future.push(self.snapshot());
                    self.restore(s);
                }
            }
            Command::Redo => {
                if let Some(s) = self.future.pop() {
                    self.history.push(self.snapshot());
                    self.restore(s);
                }
            }
            Command::Copy | Command::Cut => {
                if let Some(i) = self.editable_selection() {
                    self.clipboard = Some(self.drawing.objects[i].clone());
                    if matches!(command, Command::Cut) {
                        self.checkpoint();
                        self.drawing.objects.remove(i);
                        self.drawing.selected = None;
                    }
                }
            }
            Command::Paste | Command::Duplicate => {
                let object = if matches!(command, Command::Duplicate) {
                    self.editable_selection()
                        .map(|i| self.drawing.objects[i].clone())
                } else {
                    self.clipboard.clone()
                };
                if let Some(mut o) = object {
                    if !self.layers[self.selected_layer].locked
                        && self.layers[self.selected_layer].visible
                    {
                        self.checkpoint();
                        o.x += 16.;
                        o.y += 16.;
                        o.layer = self.selected_layer;
                        o.keyframe = self.active_keyframe(o.layer);
                        self.drawing.selected = Some(self.drawing.objects.len());
                        self.drawing.objects.push(o);
                    }
                }
            }
            Command::Delete => {
                if let Some(i) = self.editable_selection() {
                    self.checkpoint();
                    self.drawing.objects.remove(i);
                    self.drawing.selected = None;
                }
            }

            Command::AddLayer => {
                self.checkpoint();
                let i = self.layers.len();
                self.layers
                    .push(LayerState::new(&format!("Layer {}", i + 1), theme::BLUE));
                self.selected_layer = i;
            }
            Command::DeleteLayer => {
                if self.layers.len() > 1 {
                    self.checkpoint();
                    let i = self.selected_layer;
                    self.layers.remove(i);
                    self.drawing.objects.retain(|o| o.layer != i);
                    for o in &mut self.drawing.objects {
                        if o.layer > i {
                            o.layer -= 1;
                        }
                    }
                    self.selected_layer = i.min(self.layers.len() - 1);
                    self.drawing.selected = None;
                }
            }

            Command::LayerUp | Command::LayerDown => {
                let a = self.selected_layer;
                let b = if matches!(command, Command::LayerUp) {
                    a.saturating_sub(1)
                } else {
                    (a + 1).min(self.layers.len() - 1)
                };
                if a != b {
                    self.checkpoint();
                    self.layers.swap(a, b);
                    for o in &mut self.drawing.objects {
                        if o.layer == a {
                            o.layer = b;
                        } else if o.layer == b {
                            o.layer = a;
                        }
                    }
                    self.selected_layer = b;
                }
            }
            Command::Keyframe | Command::BlankKeyframe | Command::ClearKeyframe => {
                let layer = self.selected_layer;
                let frame = self.current_frame;
                if !self.layers[layer].locked {
                    self.checkpoint();
                    let previous = self.active_keyframe(layer);
                    let copies: Vec<_> = self
                        .drawing
                        .objects
                        .iter()
                        .filter(|o| o.layer == layer && o.keyframe == previous)
                        .cloned()
                        .collect();
                    if matches!(command, Command::ClearKeyframe) && frame != 1 {
                        self.layers[layer].keyframes.retain(|f| *f != frame);
                        self.drawing
                            .objects
                            .retain(|o| o.layer != layer || o.keyframe != frame);
                    } else if !matches!(command, Command::ClearKeyframe) {
                        let exists = self.layers[layer].keyframes.contains(&frame);
                        if matches!(command, Command::BlankKeyframe) {
                            self.drawing
                                .objects
                                .retain(|o| o.layer != layer || o.keyframe != frame);
                        }
                        if !exists {
                            self.layers[layer].keyframes.push(frame);
                            self.layers[layer].keyframes.sort_unstable();
                            if matches!(command, Command::Keyframe) {
                                for mut o in copies {
                                    o.keyframe = frame;
                                    self.drawing.objects.push(o);
                                }
                            }
                        }
                    }
                    self.drawing.selected = None;
                }
            }
            Command::Extend => {
                self.checkpoint();
                self.frame_count = (self.frame_count + 12).min(10000);
            }
            Command::Grid => self.grid = !self.grid,
            Command::Onion => self.onion = !self.onion,
            Command::Loop => self.looping = !self.looping,
            Command::Fit => {
                let b = self.drawing.bounds.get();
                self.zoom = (((f32::from(b.size.width) - 48.) / self.stage_size[0])
                    .min((f32::from(b.size.height) - 48.) / self.stage_size[1])
                    * 100.)
                    .clamp(10., 400.) as u32;
                self.drawing.pan = [0., 0.];
            }
            Command::ZoomIn => self.zoom = (self.zoom + 10).min(400),
            Command::ZoomOut => self.zoom = self.zoom.saturating_sub(10).max(10),
            Command::Properties => self.inspector_tab = InspectorTab::Properties,
            Command::Library => self.inspector_tab = InspectorTab::Library,
            Command::Align => self.inspector_tab = InspectorTab::Align,

            Command::Help => self.modal = Some(panels::Modal::help()),
            Command::SaveSymbol => {
                if let Some(i) = self.editable_selection() {
                    self.checkpoint();
                    self.symbols.push(self.drawing.objects[i].clone());
                    self.inspector_tab = InspectorTab::Library;
                }
            }
            Command::PlaceSymbol(i) => {
                if let Some(mut o) = self.symbols.get(i).cloned() {
                    if !self.layers[self.selected_layer].locked {
                        self.checkpoint();
                        o.layer = self.selected_layer;
                        o.keyframe = self.active_keyframe(o.layer);
                        o.x = 100.;
                        o.y = 100.;
                        self.drawing.selected = Some(self.drawing.objects.len());
                        self.drawing.objects.push(o);
                    }
                }
            }
            Command::Left
            | Command::Right
            | Command::Top
            | Command::Bottom
            | Command::CenterX
            | Command::CenterY
            | Command::Front
            | Command::Back => {
                if let Some(i) = self.editable_selection() {
                    self.checkpoint();
                    let o = &mut self.drawing.objects[i];
                    match command {
                        Command::Left => o.x = 0.,
                        Command::Right => o.x = self.stage_size[0] - o.w,
                        Command::Top => o.y = 0.,
                        Command::Bottom => o.y = self.stage_size[1] - o.h,
                        Command::CenterX => o.x = (self.stage_size[0] - o.w) / 2.,
                        Command::CenterY => o.y = (self.stage_size[1] - o.h) / 2.,
                        Command::Front => {
                            let o = self.drawing.objects.remove(i);
                            self.drawing.objects.push(o);
                            self.drawing.selected = Some(self.drawing.objects.len() - 1);
                        }
                        Command::Back => {
                            let o = self.drawing.objects.remove(i);
                            self.drawing.objects.insert(0, o);
                            self.drawing.selected = Some(0);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn command_button(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        command: Command,
        cx: &Context<Self>,
    ) -> Button {
        Button::new(id)
            .ghost()
            .compact()
            .icon(icon)
            .tooltip(label)
            .accessibility_label(label)
            .on_click(cx.listener(move |this, _, window, cx| this.command(command, window, cx)))
    }

    pub(super) fn popup(
        &self,
        mut menu: PopupMenu,
        commands: &[(&str, Command)],
        cx: &Context<Self>,
    ) -> PopupMenu {
        for (label, command) in commands {
            let command = *command;
            let weak = cx.weak_entity();
            let enabled = match command {
                Command::Undo => !self.history.is_empty(),
                Command::Redo => !self.future.is_empty(),
                Command::Copy
                | Command::Cut
                | Command::Duplicate
                | Command::Delete
                | Command::ObjectSettings
                | Command::SaveSymbol
                | Command::Front
                | Command::Back => self.editable_selection().is_some(),
                Command::Paste => {
                    self.clipboard.is_some() && !self.layers[self.selected_layer].locked
                }
                Command::DeleteLayer => self.layers.len() > 1,
                _ => true,
            };
            menu = menu.item(
                PopupMenuItem::new(label.to_string())
                    .disabled(!enabled)
                    .on_click(move |_, window, cx| {
                        let _ = weak.update(cx, |this, cx| this.command(command, window, cx));
                    }),
            );
        }
        if let Some(focus) = &self.focus {
            menu = menu.action_context(focus.clone());
        }
        menu
    }

    pub(super) fn open_document(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open Popcorn document".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.first() {
                    let path = path.clone();
                    let read_path = path.clone();
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            std::fs::read_to_string(read_path)
                                .map_err(|e| e.to_string())
                                .and_then(|text| {
                                    serde_json::from_str::<Snapshot>(&text)
                                        .map_err(|e| e.to_string())
                                })
                                .and_then(|s| {
                                    s.validate()?;
                                    Ok(s)
                                })
                        })
                        .await;
                    let _ = this.update(cx, |editor, cx| {
                        match result {
                            Ok(s) => {
                                editor.checkpoint();
                                editor.restore(s);
                                editor.path = Some(path);
                                editor.saved =
                                    serde_json::to_string(&editor.snapshot()).unwrap_or_default();
                                editor.status = "Document opened".into();
                            }
                            Err(e) => editor.status = format!("Open failed: {e}"),
                        }
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    pub(super) fn save_document(&mut self, save_as: bool, cx: &mut Context<Self>) {
        let data = match serde_json::to_string_pretty(&self.snapshot()) {
            Ok(data) => data,
            Err(e) => {
                self.status = e.to_string();
                return;
            }
        };
        let snapshot = serde_json::to_string(&self.snapshot()).unwrap_or_default();
        let existing = if save_as { None } else { self.path.clone() };
        let receiver = if existing.is_none() {
            Some(cx.prompt_for_new_path(
                &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                Some("Untitled.pop"),
            ))
        } else {
            None
        };
        cx.spawn(async move |this, cx| {
            let path = match (existing, receiver) {
                (Some(path), _) => Some(path),
                (_, Some(rx)) => rx.await.ok().and_then(Result::ok).flatten(),
                _ => None,
            };
            if let Some(path) = path {
                let write_path = path.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        // Write beside the destination, then rename so a failed write keeps the old document.
                        let temp =
                            write_path.with_extension(format!("pop-{}.tmp", std::process::id()));
                        std::fs::write(&temp, data)
                            .and_then(|_| std::fs::rename(&temp, &write_path))
                            .map_err(|e| e.to_string())
                    })
                    .await;
                let _ = this.update(cx, |editor, cx| {
                    match result {
                        Ok(()) => {
                            editor.path = Some(path);
                            editor.saved = snapshot;
                            editor.status = "Document saved".into();
                        }
                        Err(e) => editor.status = format!("Save failed: {e}"),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }
}
