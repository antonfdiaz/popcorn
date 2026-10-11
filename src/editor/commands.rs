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
    DeleteFrames,
    MoveFramesLeft,
    MoveFramesRight,
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
    fn selected_frames_move_and_delete_keyframes_with_their_objects() {
        let mut e = seeded_editor();
        e.current_frame = 8;
        e.apply_edit(Command::Keyframe);
        e.current_frame = 12;
        e.apply_edit(Command::Keyframe);

        e.select_timeline_frame(0, 8, false);
        e.select_timeline_frame(0, 12, true);
        assert_eq!(e.selected_frames.len(), 5);

        e.apply_edit(Command::MoveFramesRight);
        assert_eq!(e.layers[0].keyframes, vec![1, 9, 13]);
        assert!(e.drawing.objects.iter().any(|object| object.keyframe == 9));
        assert!(e.drawing.objects.iter().any(|object| object.keyframe == 13));

        e.apply_edit(Command::DeleteFrames);
        assert_eq!(e.layers[0].keyframes, vec![1]);
        assert_eq!(e.drawing.objects.len(), 1);

        e.select_timeline_frame(0, 1, false);
        e.apply_edit(Command::DeleteFrames);
        assert_eq!(e.layers[0].keyframes, vec![1]);
        assert!(e.drawing.objects.is_empty());
    }

    #[::core::prelude::v1::test]
    fn multiple_layers_move_as_a_block_and_delete_remaps_objects() {
        let mut e = seeded_editor();
        e.apply_edit(Command::AddLayer);
        e.apply_edit(Command::AddLayer);
        e.drawing.objects.extend([
            Object {
                layer: 1,
                keyframe: 1,
                x: 10.,
                y: 10.,
                w: 20.,
                h: 20.,
                kind: workspace::Kind::Ellipse,
                fill: theme::BLUE,
                rotation: 0.,
            },
            Object {
                layer: 2,
                keyframe: 1,
                x: 20.,
                y: 20.,
                w: 20.,
                h: 20.,
                kind: workspace::Kind::Ellipse,
                fill: theme::GREEN,
                rotation: 0.,
            },
        ]);

        e.set_layer_selection([1, 2]);
        e.selected_layer = 1;
        e.apply_edit(Command::LayerUp);
        assert_eq!(e.layers[0].name, "Layer 2");
        assert_eq!(e.layers[1].name, "Layer 3");
        assert_eq!(e.layers[2].name, "Layer 1");
        assert!(e.drawing.objects.iter().any(|object| object.layer == 0));
        assert!(e.drawing.objects.iter().any(|object| object.layer == 1));

        e.set_layer_selection([0, 1]);
        e.selected_layer = 0;
        e.apply_edit(Command::DeleteLayer);
        assert_eq!(e.layers.len(), 1);
        assert!(e.drawing.objects.iter().all(|object| object.layer == 0));
        assert_eq!(e.selected_layers, vec![0]);
    }

    #[::core::prelude::v1::test]
    fn context_click_preserves_an_existing_timeline_selection() {
        let mut e = Editor::default();
        e.apply_edit(Command::AddLayer);
        e.set_layer_selection([0, 1]);
        e.selected_layer = 0;
        e.context_select_timeline_layer(1, false);
        assert_eq!(e.selected_layers, vec![0, 1]);

        e.select_timeline_frame(0, 4, false);
        e.select_timeline_frame(0, 8, true);
        e.context_select_timeline_frame(0, 6, false);
        assert_eq!(e.selected_frames.len(), 5);

        e.context_select_timeline_frame(1, 6, false);
        assert_eq!(e.selected_frames, vec![(1, 6)]);
    }

    #[::core::prelude::v1::test]
    fn dragging_a_selected_keyframe_moves_the_selected_group() {
        let mut e = seeded_editor();
        e.current_frame = 8;
        e.apply_edit(Command::Keyframe);
        e.current_frame = 12;
        e.apply_edit(Command::Keyframe);
        e.select_timeline_frame(0, 8, false);
        e.select_timeline_frame(0, 12, true);

        e.begin_timeline_drag(0, 8, false);
        e.update_timeline_drag(0, 9);
        e.finish_timeline_drag();

        assert_eq!(e.layers[0].keyframes, vec![1, 9, 13]);
        assert_eq!(e.selected_frames.first(), Some(&(0, 9)));
    }

    #[::core::prelude::v1::test]
    fn dragging_the_first_keyframe_moves_its_content_and_keeps_a_blank_root() {
        let mut e = seeded_editor();
        e.select_timeline_frame(0, 1, false);
        e.begin_timeline_drag(0, 1, false);
        e.update_timeline_drag(0, 2);
        e.finish_timeline_drag();

        assert_eq!(e.layers[0].keyframes, vec![1, 2]);
        assert!(e.drawing.objects.iter().all(|object| object.keyframe == 2));
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
                    || !o.valid_geometry()
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
        self.set_layer_selection([self.selected_layer]);
        self.clear_timeline_selection();
        self.clear_selection();
        self.drawing.drag = None;
        self.text_editing = None;
    }

    pub(super) fn active_keyframe(&self, layer: usize) -> usize {
        self.active_keyframe_at(layer, self.current_frame)
    }

    fn active_keyframe_at(&self, layer: usize, frame: usize) -> usize {
        self.layers[layer]
            .keyframes
            .iter()
            .copied()
            .take_while(|f| *f <= frame)
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

    pub(super) fn selected_indices(&self) -> Vec<usize> {
        let selection = if self.drawing.selection.is_empty() {
            self.drawing.selected.into_iter().collect()
        } else {
            self.drawing.selection.clone()
        };
        selection
            .into_iter()
            .filter(|i| {
                self.drawing
                    .objects
                    .get(*i)
                    .is_some_and(|o| self.object_visible(o))
            })
            .collect()
    }

    pub(super) fn editable_selections(&self) -> Vec<usize> {
        self.selected_indices()
            .into_iter()
            .filter(|i| !self.layers[self.drawing.objects[*i].layer].locked)
            .collect()
    }

    pub(super) fn set_selection(&mut self, indices: impl IntoIterator<Item = usize>) {
        let mut selection = indices
            .into_iter()
            .filter(|i| *i < self.drawing.objects.len())
            .collect::<Vec<_>>();
        let mut seen = std::collections::HashSet::new();
        selection.retain(|i| seen.insert(*i));
        self.drawing.selected = selection.first().copied();
        self.drawing.selection = selection;
    }

    pub(super) fn clear_selection(&mut self) {
        self.drawing.selected = None;
        self.drawing.selection.clear();
    }

    pub(super) fn is_selected(&self, index: usize) -> bool {
        if self.drawing.selection.is_empty() {
            self.drawing.selected == Some(index)
        } else {
            self.drawing.selection.contains(&index)
        }
    }

    pub(super) fn selected_layer_indices(&self) -> Vec<usize> {
        if self.selected_layers.is_empty() || !self.selected_layers.contains(&self.selected_layer) {
            vec![self.selected_layer]
        } else {
            self.selected_layers.clone()
        }
    }

    pub(super) fn set_layer_selection(&mut self, indices: impl IntoIterator<Item = usize>) {
        let mut selection = indices
            .into_iter()
            .filter(|index| *index < self.layers.len())
            .collect::<Vec<_>>();
        let mut seen = std::collections::HashSet::new();
        selection.retain(|index| seen.insert(*index));
        if selection.is_empty() && !self.layers.is_empty() {
            selection.push(self.selected_layer.min(self.layers.len() - 1));
        }
        self.selected_layers = selection;
        self.selected_layer = self.selected_layers.first().copied().unwrap_or_default();
    }

    pub(super) fn is_layer_selected(&self, index: usize) -> bool {
        self.selected_layer_indices().contains(&index)
    }

    pub(super) fn is_frame_selected(&self, layer: usize, frame: usize) -> bool {
        self.selected_frames.contains(&(layer, frame))
    }

    pub(super) fn select_timeline_layer(&mut self, layer: usize, shift: bool) {
        if layer >= self.layers.len() {
            return;
        }
        if shift {
            if let Some(anchor) = self.layer_selection_anchor {
                let start = anchor.min(layer);
                let end = anchor.max(layer);
                self.set_layer_selection(start..=end);
            } else {
                self.set_layer_selection([layer]);
            }
        } else {
            self.set_layer_selection([layer]);
        }
        self.selected_layer = layer;
        self.layer_selection_anchor = Some(layer);
        self.clear_selection();
        self.clear_timeline_selection();
    }

    pub(super) fn context_select_timeline_layer(&mut self, layer: usize, shift: bool) {
        let preserve = !shift && self.is_layer_selected(layer);
        if preserve {
            self.selected_layer = layer;
            self.layer_selection_anchor = Some(layer);
            self.clear_selection();
            self.clear_timeline_selection();
        } else {
            self.select_timeline_layer(layer, shift);
        }
    }

    pub(super) fn select_timeline_frame(&mut self, layer: usize, frame: usize, shift: bool) {
        if layer >= self.layers.len() || !(1..=self.frame_count).contains(&frame) {
            return;
        }
        if shift {
            if let Some((anchor_layer, anchor_frame)) = self.frame_selection_anchor {
                if anchor_layer == layer {
                    let start = anchor_frame.min(frame);
                    let end = anchor_frame.max(frame);
                    self.selected_frames
                        .retain(|(selected_layer, _)| *selected_layer != layer);
                    self.selected_frames
                        .extend((start..=end).map(|selected_frame| (layer, selected_frame)));
                } else if let Some(position) = self
                    .selected_frames
                    .iter()
                    .position(|selected| *selected == (layer, frame))
                {
                    self.selected_frames.remove(position);
                } else {
                    self.selected_frames.push((layer, frame));
                }
            } else {
                self.selected_frames.push((layer, frame));
            }
        } else {
            self.selected_frames.clear();
            self.selected_frames.push((layer, frame));
        }
        self.selected_layer = layer;
        self.current_frame = frame;
        self.frame_selection_anchor = Some((layer, frame));
        self.clear_selection();
        self.drawing.drag = None;
        self.text_editing = None;
    }

    pub(super) fn context_select_timeline_frame(
        &mut self,
        layer: usize,
        frame: usize,
        shift: bool,
    ) {
        let preserve = !shift && self.is_frame_selected(layer, frame);
        if preserve {
            self.selected_layer = layer;
            self.current_frame = frame;
            self.frame_selection_anchor = Some((layer, frame));
            self.clear_selection();
            self.drawing.drag = None;
            self.text_editing = None;
        } else {
            self.select_timeline_frame(layer, frame, shift);
        }
    }

    pub(super) fn begin_timeline_drag(&mut self, layer: usize, frame: usize, shift: bool) {
        self.timeline_pointer_handled = true;
        if layer >= self.layers.len() || !(1..=self.frame_count).contains(&frame) {
            self.timeline_drag = None;
            return;
        }

        let preserve = !shift && self.is_frame_selected(layer, frame);
        if preserve {
            self.selected_layer = layer;
            self.current_frame = frame;
            self.frame_selection_anchor = Some((layer, frame));
            self.clear_selection();
            self.drawing.drag = None;
            self.text_editing = None;
        } else {
            self.select_timeline_frame(layer, frame, shift);
        }

        let can_drag = self.layers[layer].keyframes.contains(&frame)
            && !self.layers[layer].locked
            && self
                .timeline_targets()
                .iter()
                .any(|(selected_layer, selected_frame)| {
                    *selected_layer < self.layers.len()
                        && self.layers[*selected_layer]
                            .keyframes
                            .contains(selected_frame)
                        && !self.layers[*selected_layer].locked
                });
        self.timeline_drag = can_drag.then_some(TimelineDrag { last_frame: frame });
    }

    pub(super) fn update_timeline_drag(&mut self, _layer: usize, frame: usize) {
        let Some(mut drag) = self.timeline_drag.take() else {
            return;
        };
        let delta = frame as isize - drag.last_frame as isize;
        if delta != 0 && self.move_selected_frames(delta) {
            drag.last_frame = frame;
        }
        self.timeline_drag = Some(drag);
    }

    pub(super) fn finish_timeline_drag(&mut self) {
        self.timeline_drag = None;
    }

    pub(super) fn clear_timeline_selection(&mut self) {
        self.selected_frames.clear();
        self.frame_selection_anchor = None;
        self.timeline_drag = None;
    }

    pub(super) fn timeline_targets(&self) -> Vec<(usize, usize)> {
        if self.selected_frames.is_empty() {
            vec![(self.selected_layer, self.current_frame)]
        } else {
            let mut targets = self.selected_frames.clone();
            targets.sort_unstable();
            targets.dedup();
            targets
        }
    }

    fn remap_object_layer(&mut self, a: usize, b: usize) {
        for object in &mut self.drawing.objects {
            if object.layer == a {
                object.layer = b;
            } else if object.layer == b {
                object.layer = a;
            }
        }
    }

    fn move_selected_frames(&mut self, delta: isize) -> bool {
        let selected = self.timeline_targets();
        if selected.is_empty() {
            return false;
        }
        if selected
            .iter()
            .any(|(layer, frame)| *layer >= self.layers.len() || *frame == 0)
        {
            return false;
        }

        let shifted = selected
            .iter()
            .map(|(layer, frame)| {
                let next = (*frame as isize).saturating_add(delta);
                (*layer, next)
            })
            .collect::<Vec<_>>();
        if shifted
            .iter()
            .any(|(_, frame)| *frame < 1 || *frame > self.frame_count as isize)
        {
            return false;
        }

        let mut keyframe_moves = std::collections::HashMap::new();
        for ((layer, frame), (_, next)) in selected.iter().zip(&shifted) {
            if !self.layers[*layer].locked && self.layers[*layer].keyframes.contains(frame) {
                keyframe_moves.insert((*layer, *frame), *next as usize);
            }
        }

        for ((layer, _), next) in &keyframe_moves {
            if self.layers[*layer]
                .keyframes
                .iter()
                .any(|frame| *frame == *next && !keyframe_moves.contains_key(&(*layer, *frame)))
            {
                return false;
            }
        }

        let has_keyframe_changes = !keyframe_moves.is_empty();
        if has_keyframe_changes {
            self.checkpoint();
            for layer in 0..self.layers.len() {
                let mut keyframes = Vec::new();
                for frame in &self.layers[layer].keyframes {
                    if let Some(next) = keyframe_moves.get(&(layer, *frame)) {
                        if *frame == 1 {
                            // Frame 1 is the document root. Keep its blank cell
                            // while moving its content to the dragged frame.
                            keyframes.push(1);
                        }
                        keyframes.push(*next);
                    } else {
                        keyframes.push(*frame);
                    }
                }
                keyframes.sort_unstable();
                keyframes.dedup();
                self.layers[layer].keyframes = keyframes;
            }
            for object in &mut self.drawing.objects {
                if let Some(next) = keyframe_moves.get(&(object.layer, object.keyframe)) {
                    object.keyframe = *next;
                }
            }
        }

        self.selected_frames = shifted
            .into_iter()
            .map(|(layer, frame)| (layer, frame as usize))
            .collect();
        self.current_frame = self
            .current_frame
            .saturating_add_signed(delta)
            .clamp(1, self.frame_count);
        self.frame_selection_anchor = self.selected_frames.last().copied();
        true
    }

    fn delete_selected_frames(&mut self) {
        let targets = self
            .timeline_targets()
            .into_iter()
            .filter(|(layer, frame)| {
                *layer < self.layers.len()
                    && self.layers[*layer].keyframes.contains(frame)
                    && !self.layers[*layer].locked
                    && (*frame != 1
                        || self
                            .drawing
                            .objects
                            .iter()
                            .any(|object| object.layer == *layer && object.keyframe == *frame))
            })
            .collect::<Vec<_>>();
        if targets.is_empty() {
            return;
        }

        self.checkpoint();
        let target_set = targets
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        for (layer, frame) in &target_set {
            if *frame != 1 {
                self.layers[*layer]
                    .keyframes
                    .retain(|keyframe| keyframe != frame);
            }
        }
        self.drawing
            .objects
            .retain(|object| !target_set.contains(&(object.layer, object.keyframe)));
        self.clear_timeline_selection();
    }

    pub(super) fn select_frame(&mut self, layer: usize, frame: usize) {
        self.set_layer_selection([layer]);
        self.current_frame = frame;
        self.clear_selection();
        self.drawing.drag = None;
        self.text_editing = None;
        self.clear_timeline_selection();
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
                    self.clipboard = Some(vec![Object {
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
                    }]);
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
                let indices = self.editable_selections();
                if !indices.is_empty() {
                    self.clipboard = Some(
                        indices
                            .iter()
                            .map(|i| self.drawing.objects[*i].clone())
                            .collect(),
                    );
                    if matches!(command, Command::Cut) {
                        self.checkpoint();
                        for index in indices.into_iter().rev() {
                            self.drawing.objects.remove(index);
                        }
                        self.clear_selection();
                    }
                }
            }
            Command::Paste | Command::Duplicate => {
                let objects = if matches!(command, Command::Duplicate) {
                    self.editable_selections()
                        .into_iter()
                        .map(|i| self.drawing.objects[i].clone())
                        .collect::<Vec<_>>()
                } else {
                    self.clipboard.clone().unwrap_or_default()
                };
                if !objects.is_empty() {
                    if !self.layers[self.selected_layer].locked
                        && self.layers[self.selected_layer].visible
                    {
                        self.checkpoint();
                        let mut selected = Vec::new();
                        for mut o in objects {
                            o.x += 16.;
                            o.y += 16.;
                            o.layer = self.selected_layer;
                            o.keyframe = self.active_keyframe(o.layer);
                            selected.push(self.drawing.objects.len());
                            self.drawing.objects.push(o);
                        }
                        self.set_selection(selected);
                    }
                }
            }
            Command::Delete => {
                if !self.selected_frames.is_empty() {
                    self.delete_selected_frames();
                    return;
                }
                let indices = self.editable_selections();
                if !indices.is_empty() {
                    self.checkpoint();
                    for index in indices.into_iter().rev() {
                        self.drawing.objects.remove(index);
                    }
                    self.clear_selection();
                }
            }

            Command::AddLayer => {
                self.checkpoint();
                let i = self.layers.len();
                self.layers
                    .push(LayerState::new(&format!("Layer {}", i + 1), theme::BLUE));
                self.set_layer_selection([i]);
                self.layer_selection_anchor = Some(i);
            }
            Command::DeleteLayer => {
                if self.layers.len() > 1 {
                    let mut removed = self
                        .selected_layer_indices()
                        .into_iter()
                        .filter(|index| *index < self.layers.len())
                        .collect::<Vec<_>>();
                    removed.sort_unstable();
                    removed.dedup();
                    if removed.len() >= self.layers.len() {
                        removed.retain(|index| *index != self.selected_layer);
                    }
                    if removed.is_empty() {
                        return;
                    }

                    self.checkpoint();
                    let removed_set = removed
                        .iter()
                        .copied()
                        .collect::<std::collections::HashSet<_>>();
                    let old_layers = std::mem::take(&mut self.layers);
                    let mut old_to_new = vec![0; old_layers.len()];
                    let mut new_layers = Vec::with_capacity(old_layers.len() - removed.len());
                    for (old_index, layer) in old_layers.into_iter().enumerate() {
                        if removed_set.contains(&old_index) {
                            continue;
                        }
                        old_to_new[old_index] = new_layers.len();
                        new_layers.push(layer);
                    }
                    self.layers = new_layers;
                    self.drawing
                        .objects
                        .retain(|object| !removed_set.contains(&object.layer));
                    for object in &mut self.drawing.objects {
                        object.layer = old_to_new[object.layer];
                    }

                    let new_selected = (self.selected_layer..old_to_new.len())
                        .find(|index| !removed_set.contains(index))
                        .map(|index| old_to_new[index])
                        .unwrap_or_else(|| self.layers.len() - 1);
                    self.selected_layer = new_selected;
                    self.set_layer_selection([self.selected_layer]);
                    self.layer_selection_anchor = Some(self.selected_layer);
                    self.clear_selection();
                    self.clear_timeline_selection();
                }
            }

            Command::LayerUp | Command::LayerDown => {
                let mut selected = self
                    .selected_layer_indices()
                    .into_iter()
                    .collect::<std::collections::HashSet<_>>();
                let mut primary = self.selected_layer;
                let can_move = if matches!(command, Command::LayerUp) {
                    selected
                        .iter()
                        .any(|index| *index > 0 && !selected.contains(&(*index - 1)))
                } else {
                    selected.iter().any(|index| {
                        *index + 1 < self.layers.len() && !selected.contains(&(*index + 1))
                    })
                };
                if can_move {
                    self.checkpoint();
                    if matches!(command, Command::LayerUp) {
                        for index in 1..self.layers.len() {
                            if selected.contains(&index) && !selected.contains(&(index - 1)) {
                                self.layers.swap(index - 1, index);
                                self.remap_object_layer(index - 1, index);
                                selected.remove(&index);
                                selected.insert(index - 1);
                                if primary == index {
                                    primary = index - 1;
                                }
                            }
                        }
                    } else {
                        for index in (0..self.layers.len().saturating_sub(1)).rev() {
                            if selected.contains(&index) && !selected.contains(&(index + 1)) {
                                self.layers.swap(index, index + 1);
                                self.remap_object_layer(index, index + 1);
                                selected.remove(&index);
                                selected.insert(index + 1);
                                if primary == index {
                                    primary = index + 1;
                                }
                            }
                        }
                    }
                    self.selected_layer = primary;
                    self.set_layer_selection(selected.into_iter().collect::<Vec<_>>());
                    self.selected_layer = primary;
                    self.layer_selection_anchor = Some(primary);
                }
            }
            Command::Keyframe | Command::BlankKeyframe | Command::ClearKeyframe => {
                let targets = self
                    .timeline_targets()
                    .into_iter()
                    .filter(|(layer, frame)| {
                        *layer < self.layers.len()
                            && *frame <= self.frame_count
                            && !self.layers[*layer].locked
                    })
                    .collect::<Vec<_>>();
                if targets.is_empty() {
                    return;
                }

                let changes = targets.iter().any(|(layer, frame)| {
                    let exists = self.layers[*layer].keyframes.contains(frame);
                    match command {
                        Command::Keyframe => !exists,
                        Command::BlankKeyframe => {
                            !exists
                                || self
                                    .drawing
                                    .objects
                                    .iter()
                                    .any(|o| o.layer == *layer && o.keyframe == *frame)
                        }
                        Command::ClearKeyframe => *frame != 1 && exists,
                        _ => false,
                    }
                });
                if !changes {
                    return;
                }

                self.checkpoint();
                for (layer, frame) in targets {
                    let exists = self.layers[layer].keyframes.contains(&frame);
                    if matches!(command, Command::ClearKeyframe) {
                        if frame != 1 {
                            self.layers[layer].keyframes.retain(|f| *f != frame);
                            self.drawing
                                .objects
                                .retain(|o| o.layer != layer || o.keyframe != frame);
                        }
                        continue;
                    }

                    if matches!(command, Command::BlankKeyframe) {
                        self.drawing
                            .objects
                            .retain(|o| o.layer != layer || o.keyframe != frame);
                    }
                    if !exists {
                        let copies = if matches!(command, Command::Keyframe) {
                            let previous = self.active_keyframe_at(layer, frame);
                            self.drawing
                                .objects
                                .iter()
                                .filter(|o| o.layer == layer && o.keyframe == previous)
                                .cloned()
                                .collect::<Vec<_>>()
                        } else {
                            Vec::new()
                        };
                        self.layers[layer].keyframes.push(frame);
                        self.layers[layer].keyframes.sort_unstable();
                        for mut object in copies {
                            object.keyframe = frame;
                            self.drawing.objects.push(object);
                        }
                    }
                }
                self.clear_selection();
            }
            Command::DeleteFrames => self.delete_selected_frames(),
            Command::MoveFramesLeft => {
                self.move_selected_frames(-1);
            }
            Command::MoveFramesRight => {
                self.move_selected_frames(1);
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
                        let index = self.drawing.objects.len();
                        self.drawing.objects.push(o);
                        self.set_selection([index]);
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
                let indices = self.editable_selections();
                if !indices.is_empty() {
                    self.checkpoint();
                    match command {
                        Command::Left => {
                            for i in indices {
                                self.drawing.objects[i].x = 0.;
                            }
                        }
                        Command::Right => {
                            for i in indices {
                                let object = &mut self.drawing.objects[i];
                                object.x = self.stage_size[0] - object.w;
                            }
                        }
                        Command::Top => {
                            for i in indices {
                                self.drawing.objects[i].y = 0.;
                            }
                        }
                        Command::Bottom => {
                            for i in indices {
                                let object = &mut self.drawing.objects[i];
                                object.y = self.stage_size[1] - object.h;
                            }
                        }
                        Command::CenterX => {
                            for i in indices {
                                let object = &mut self.drawing.objects[i];
                                object.x = (self.stage_size[0] - object.w) / 2.;
                            }
                        }
                        Command::CenterY => {
                            for i in indices {
                                let object = &mut self.drawing.objects[i];
                                object.y = (self.stage_size[1] - object.h) / 2.;
                            }
                        }
                        Command::Front => {
                            let selected = indices
                                .iter()
                                .copied()
                                .collect::<std::collections::HashSet<_>>();
                            let mut moved = Vec::new();
                            let mut remaining = Vec::new();
                            for (index, object) in self.drawing.objects.drain(..).enumerate() {
                                if selected.contains(&index) {
                                    moved.push(object);
                                } else {
                                    remaining.push(object);
                                }
                            }
                            let start = remaining.len();
                            remaining.extend(moved);
                            self.drawing.objects = remaining;
                            self.set_selection(start..self.drawing.objects.len());
                        }
                        Command::Back => {
                            let selected = indices
                                .iter()
                                .copied()
                                .collect::<std::collections::HashSet<_>>();
                            let mut moved = Vec::new();
                            let mut remaining = Vec::new();
                            for (index, object) in self.drawing.objects.drain(..).enumerate() {
                                if selected.contains(&index) {
                                    moved.push(object);
                                } else {
                                    remaining.push(object);
                                }
                            }
                            moved.extend(remaining);
                            self.drawing.objects = moved;
                            self.set_selection(0..indices.len());
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
                | Command::Back => {
                    self.editable_selection().is_some()
                        || (matches!(command, Command::Delete) && !self.selected_frames.is_empty())
                }
                Command::Paste => {
                    self.clipboard.is_some() && !self.layers[self.selected_layer].locked
                }
                Command::DeleteLayer => self.layers.len() > 1,
                Command::DeleteFrames => {
                    self.timeline_targets().iter().any(|(layer, frame)| {
                        *layer < self.layers.len()
                            && self.layers[*layer].keyframes.contains(frame)
                            && !self.layers[*layer].locked
                            && (*frame != 1
                                || self.drawing.objects.iter().any(|object| {
                                    object.layer == *layer && object.keyframe == *frame
                                }))
                    })
                }
                Command::MoveFramesLeft => self.timeline_targets().iter().any(|(layer, frame)| {
                    *layer < self.layers.len()
                        && *frame > 1
                        && !self.layers[*layer].locked
                        && self.layers[*layer].keyframes.contains(frame)
                }),
                Command::MoveFramesRight => self.timeline_targets().iter().any(|(layer, frame)| {
                    *layer < self.layers.len()
                        && *frame < self.frame_count
                        && !self.layers[*layer].locked
                        && self.layers[*layer].keyframes.contains(frame)
                }),
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
