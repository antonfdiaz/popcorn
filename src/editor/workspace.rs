use super::*;
use std::sync::Arc;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) enum Kind {
    Rectangle,
    Ellipse,
    Pencil(Vec<[f32; 2]>),
    Text(String),
    Image { bytes: Vec<u8>, format: String },
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Object {
    pub(super) kind: Kind,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) w: f32,
    pub(super) h: f32,
    pub(super) layer: usize,
    pub(super) keyframe: usize,
    pub(super) fill: u32,
}

pub(super) enum Drag {
    Draw(usize, [f32; 2]),
    Move(usize, [f32; 2], [f32; 2]),
    Resize(usize, [f32; 2]),
    Pan([f32; 2], [f32; 2]),
}

pub(super) struct Drawing {
    pub(super) objects: Vec<Object>,
    pub(super) selected: Option<usize>,
    pub(super) drag: Option<Drag>,
    pub(super) pan: [f32; 2],
    pub(super) bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl Default for Drawing {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            selected: None,
            drag: None,
            pan: [0., 0.],
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }
}

impl Editor {
    fn render_object(o: &Object, scale: f32, selected: bool) -> Div {
        let base = div()
            .absolute()
            .left(px(o.x * scale))
            .top(px(o.y * scale))
            .w(px(o.w * scale))
            .h(px(o.h * scale))
            .when(selected, |d| {
                d.border_2().border_color(theme::color(theme::BLUE))
            });
        match &o.kind {
            Kind::Rectangle => base.bg(theme::color(o.fill)),
            Kind::Ellipse => base.rounded_full().bg(theme::color(o.fill)),
            Kind::Text(text) => base
                .text_color(theme::color(o.fill))
                .text_size(px(24. * scale))
                .child(text.clone()),
            Kind::Image { bytes, format } => {
                let image_format = ImageFormat::from_mime_type(format).unwrap_or(ImageFormat::Png);
                base.child(
                    img(ImageSource::Image(Arc::new(Image::from_bytes(
                        image_format,
                        bytes.clone(),
                    ))))
                    .size_full()
                    .object_fit(ObjectFit::Contain),
                )
            }
            Kind::Pencil(points) => {
                let points = points.clone();
                let fill = o.fill;
                base.child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, window, _| {
                            let mut path = PathBuilder::stroke(px(3. * scale));
                            for (i, p) in points.iter().enumerate() {
                                let p = bounds.origin + point(px(p[0] * scale), px(p[1] * scale));
                                if i == 0 {
                                    path.move_to(p);
                                } else {
                                    path.line_to(p);
                                }
                            }
                            if let Ok(path) = path.build() {
                                window.paint_path(path, theme::color(fill));
                            }
                        },
                    )
                    .size_full(),
                )
            }
        }
    }

    fn hit_test(&self, p: [f32; 2]) -> Option<usize> {
        self.drawing
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                self.object_visible(o)
                    && !self.layers[o.layer].locked
                    && p[0] >= o.x
                    && p[1] >= o.y
                    && p[0] <= o.x + o.w
                    && p[1] <= o.y + o.h
            })
            .min_by_key(|(i, o)| (o.layer, std::cmp::Reverse(*i)))
            .map(|(i, _)| i)
    }

    pub(super) fn render_workspace(&self, cx: &mut Context<Self>) -> AnyElement {
        let bounds = self.drawing.bounds.clone();
        let entity = cx.weak_entity();
        let scale = self.zoom as f32 / 100.;
        let origin = self.stage_origin();
        let mut objects: Vec<_> = self
            .drawing
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| self.object_visible(o))
            .collect();
        objects.sort_by_key(|(i, o)| (std::cmp::Reverse(o.layer), *i));
        div()
            .relative()
            .flex_1()
            .h_full()
            .min_w_0()
            .overflow_hidden()
            .bg(theme::color(0x181a20))
            .child(
                div()
                    .absolute()
                    .top(px(48.))
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .id("drawing-viewport")
                    .child(
                        canvas(
                            move |rect, _, cx| {
                                if bounds.get() != rect {
                                    bounds.set(rect);
                                    let entity = entity.clone();
                                    cx.defer(move |cx| {
                                        let _ = entity.update(cx, |_, cx| cx.notify());
                                    });
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(origin[0]))
                            .top(px(origin[1]))
                            .w(px(self.stage_size[0] * scale))
                            .h(px(self.stage_size[1] * scale))
                            .bg(theme::color(theme::STAGE))
                            .shadow_lg()
                            .overflow_hidden()
                            .when(self.grid, |stage| {
                                stage
                                    .children((0..=(self.stage_size[0] / 20.) as usize).map(|i| {
                                        div()
                                            .absolute()
                                            .left(px(i as f32 * 20. * scale))
                                            .top_0()
                                            .bottom_0()
                                            .w_px()
                                            .bg(theme::color(0xdde1e8))
                                    }))
                                    .children((0..=(self.stage_size[1] / 20.) as usize).map(|i| {
                                        div()
                                            .absolute()
                                            .top(px(i as f32 * 20. * scale))
                                            .left_0()
                                            .right_0()
                                            .h_px()
                                            .bg(theme::color(0xdde1e8))
                                    }))
                            })
                            .when(self.onion && self.current_frame > 1, |stage| {
                                stage.children(
                                    self.drawing
                                        .objects
                                        .iter()
                                        .filter(|o| {
                                            self.layers[o.layer].visible
                                                && o.keyframe
                                                    == self.layers[o.layer]
                                                        .keyframes
                                                        .iter()
                                                        .copied()
                                                        .take_while(|f| *f < self.current_frame)
                                                        .last()
                                                        .unwrap_or(1)
                                                && o.keyframe != self.active_keyframe(o.layer)
                                        })
                                        .map(|o| Self::render_object(o, scale, false).opacity(0.2)),
                                )
                            })
                            .children(objects.into_iter().map(|(i, o)| {
                                Self::render_object(o, scale, self.drawing.selected == Some(i))
                            })),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseDownEvent, window, cx| {
                            if let Some(focus) = &this.focus {
                                window.focus(focus, cx);
                            }
                            this.pointer_down(e.position, e.modifiers.alt);
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                        if !e.dragging() {
                            this.drawing.drag = None;
                            return;
                        }
                        this.pointer_move(e.position, e.modifiers.shift);
                        cx.notify();
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.drawing.drag = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, e: &MouseDownEvent, window, cx| {
                            if let Some(f) = &this.focus {
                                window.focus(f, cx);
                            }
                            this.drawing.selected = this.hit_test(this.document_point(e.position));
                            if let Some(i) = this.drawing.selected {
                                this.selected_layer = this.drawing.objects[i].layer;
                            }
                            cx.notify();
                        }),
                    )
                    .can_drop(|value, _, _| value.downcast_ref::<ExternalPaths>().is_some())
                    .drag_over::<ExternalPaths>(|style, _, _, _| {
                        style.border_2().border_color(theme::color(theme::ACCENT))
                    })
                    .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                        let position = this.document_point(window.mouse_position());
                        this.import_external_images(paths, position);
                        cx.notify();
                    }))
                    .test_support()
                    .context_menu({
                        let weak = cx.weak_entity();
                        move |menu, _, cx| {
                            weak.update(cx, |editor, cx| {
                                editor.popup(
                                    menu,
                                    &[
                                        ("Undo", Command::Undo),
                                        ("Redo", Command::Redo),
                                        ("Cut", Command::Cut),
                                        ("Copy", Command::Copy),
                                        ("Paste", Command::Paste),
                                        ("Duplicate", Command::Duplicate),
                                        ("Delete", Command::Delete),
                                        ("Properties…", Command::ObjectSettings),
                                        ("Store in library", Command::SaveSymbol),
                                        ("Bring to front", Command::Front),
                                        ("Send to back", Command::Back),
                                    ],
                                    cx,
                                )
                            })
                            .expect("editor alive")
                        }
                    }),
            )
            .child(self.render_canvas_toolbar(cx))
            .into_any_element()
    }

    fn stage_origin(&self) -> [f32; 2] {
        let b = self.drawing.bounds.get();
        let s = self.zoom as f32 / 100.;
        [
            (f32::from(b.size.width) - self.stage_size[0] * s) / 2. + self.drawing.pan[0],
            (f32::from(b.size.height) - self.stage_size[1] * s) / 2. + self.drawing.pan[1],
        ]
    }

    fn document_point(&self, p: Point<Pixels>) -> [f32; 2] {
        let b = self.drawing.bounds.get();
        let o = self.stage_origin();
        let s = self.zoom as f32 / 100.;
        [
            (f32::from(p.x - b.origin.x) - o[0]) / s,
            (f32::from(p.y - b.origin.y) - o[1]) / s,
        ]
    }

    fn import_external_images(&mut self, paths: &ExternalPaths, position: [f32; 2]) {
        if self.layers[self.selected_layer].locked || !self.layers[self.selected_layer].visible {
            self.status = "The selected layer is locked or hidden".into();
            return;
        }

        let format_for = |path: &std::path::Path| match path
            .extension()?
            .to_string_lossy()
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => Some("image/png"),
            "jpg" | "jpeg" => Some("image/jpeg"),
            "webp" => Some("image/webp"),
            "gif" => Some("image/gif"),
            "svg" => Some("image/svg+xml"),
            "bmp" => Some("image/bmp"),
            "tif" | "tiff" => Some("image/tiff"),
            "ico" => Some("image/ico"),
            _ => None,
        };
        let mut imported = 0;
        for (offset, path) in paths.paths().iter().enumerate() {
            let Some(format) = format_for(path) else {
                continue;
            };
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            if bytes.is_empty() {
                continue;
            }
            if imported == 0 {
                self.checkpoint();
            }
            let w = 320.;
            let h = 200.;
            let x = (position[0] - w / 2. + offset as f32 * 20.)
                .clamp(0., (self.stage_size[0] - w).max(0.));
            let y = (position[1] - h / 2. + offset as f32 * 20.)
                .clamp(0., (self.stage_size[1] - h).max(0.));
            let index = self.drawing.objects.len();
            self.drawing.objects.push(Object {
                kind: Kind::Image {
                    bytes,
                    format: format.into(),
                },
                x,
                y,
                w,
                h,
                layer: self.selected_layer,
                keyframe: self.active_keyframe(self.selected_layer),
                fill: self.fill,
            });
            self.drawing.selected = Some(index);
            imported += 1;
        }
        self.status = if imported == 0 {
            "Drop a supported image (PNG, JPEG, WebP, GIF, SVG, BMP, TIFF or ICO)".into()
        } else {
            format!(
                "Imported {imported} image{}",
                if imported == 1 { "" } else { "s" }
            )
        };
    }

    fn pointer_down(&mut self, pos: Point<Pixels>, alt: bool) {
        let p = self.document_point(pos);
        if self.selected_tool == Tool::Hand {
            self.drawing.drag = Some(Drag::Pan([pos.x.into(), pos.y.into()], self.drawing.pan));
            return;
        }
        if self.selected_tool == Tool::Zoom {
            self.zoom = if alt {
                self.zoom.saturating_sub(25).max(25)
            } else {
                (self.zoom + 25).min(400)
            };
            return;
        }
        if matches!(self.selected_tool, Tool::Select | Tool::Transform) {
            let hit = self.hit_test(p);
            self.drawing.selected = hit;
            if let Some(i) = hit {
                self.checkpoint();
                let o = &self.drawing.objects[i];
                self.selected_layer = o.layer;
                self.drawing.drag = Some(if self.selected_tool == Tool::Transform {
                    Drag::Resize(i, [o.x, o.y])
                } else {
                    Drag::Move(i, p, [o.x, o.y])
                });
            }
            return;
        }
        if p[0] < 0.
            || p[1] < 0.
            || p[0] > self.stage_size[0]
            || p[1] > self.stage_size[1]
            || self.layers[self.selected_layer].locked
            || !self.layers[self.selected_layer].visible
        {
            return;
        }
        self.checkpoint();
        let kind = match self.selected_tool {
            Tool::Ellipse => Kind::Ellipse,
            Tool::Pencil => Kind::Pencil(vec![[0., 0.]]),
            Tool::Text => Kind::Text(String::new()),
            _ => Kind::Rectangle,
        };
        let text = matches!(kind, Kind::Text(_));
        let i = self.drawing.objects.len();
        self.drawing.objects.push(Object {
            kind,
            x: p[0],
            y: p[1],
            w: if text { 200. } else { 1. },
            h: if text { 36. } else { 1. },
            layer: self.selected_layer,
            keyframe: self.active_keyframe(self.selected_layer),
            fill: self.fill,
        });
        self.drawing.selected = Some(i);
        self.drawing.drag = if text { None } else { Some(Drag::Draw(i, p)) };
    }

    fn pointer_move(&mut self, pos: Point<Pixels>, shift: bool) {
        let p = self.document_point(pos);
        match &self.drawing.drag {
            Some(Drag::Pan(start, initial)) => {
                self.drawing.pan = [
                    initial[0] + f32::from(pos.x) - start[0],
                    initial[1] + f32::from(pos.y) - start[1],
                ]
            }
            Some(Drag::Move(i, start, initial)) => {
                let o = &mut self.drawing.objects[*i];
                o.x = initial[0] + p[0] - start[0];
                o.y = initial[1] + p[1] - start[1];
            }
            Some(Drag::Resize(i, start)) => {
                let o = &mut self.drawing.objects[*i];
                o.w = (p[0] - start[0]).max(2.);
                o.h = if shift {
                    o.w
                } else {
                    (p[1] - start[1]).max(2.)
                };
            }
            Some(Drag::Draw(i, start)) => {
                let o = &mut self.drawing.objects[*i];
                if let Kind::Pencil(points) = &mut o.kind {
                    points.push([p[0] - o.x, p[1] - o.y]);
                    o.w = o.w.max(p[0] - o.x + 4.);
                    o.h = o.h.max(p[1] - o.y + 4.);
                } else {
                    let dx = p[0] - start[0];
                    let dy = if shift {
                        dx.abs() * (p[1] - start[1]).signum()
                    } else {
                        p[1] - start[1]
                    };
                    o.x = start[0].min(start[0] + dx);
                    o.y = start[1].min(start[1] + dy);
                    o.w = dx.abs().max(1.);
                    o.h = dy.abs().max(1.);
                }
            }
            None => {}
        }
    }

    pub(super) fn edit_text(&mut self, event: &KeyDownEvent) -> bool {
        if self.modal.is_some()
            || self.selected_tool != Tool::Text
            || event.keystroke.modifiers.platform
            || event.keystroke.modifiers.control
            || event.keystroke.modifiers.alt
        {
            return false;
        }
        let Some(i) = self.editable_selection() else {
            return false;
        };
        if !matches!(self.drawing.objects[i].kind, Kind::Text(_)) {
            return false;
        }
        self.checkpoint();
        let o = &mut self.drawing.objects[i];
        if self.layers[o.layer].locked {
            return false;
        }
        if let Kind::Text(text) = &mut o.kind {
            if event.keystroke.key == "backspace" {
                text.pop();
                return true;
            }
            if let Some(input) = &event.keystroke.key_char {
                text.push_str(input);
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> Editor {
        let editor = Editor::default();
        editor
            .drawing
            .bounds
            .set(Bounds::new(point(px(0.), px(0.)), size(px(800.), px(450.))));
        editor
    }

    #[::core::prelude::v1::test]
    fn ellipse_drag_in_reverse_normalizes_bounds() {
        let mut editor = editor();
        editor.selected_tool = Tool::Ellipse;
        editor.pointer_down(point(px(200.), px(180.)), false);
        editor.pointer_move(point(px(100.), px(80.)), false);
        let object = editor.drawing.objects.last().unwrap();
        assert!(matches!(object.kind, Kind::Ellipse));
        assert_eq!(
            [object.x, object.y, object.w, object.h],
            [100., 80., 100., 100.]
        );
    }

    #[::core::prelude::v1::test]
    fn locked_layer_rejects_drawing_and_selection() {
        let mut editor = editor();
        editor.selected_layer = 0;
        editor.layers[0].locked = true;
        editor.selected_tool = Tool::Rectangle;
        editor.pointer_down(point(px(320.), px(150.)), false);
        assert!(editor.drawing.objects.is_empty());
        editor.selected_tool = Tool::Select;
        editor.pointer_down(point(px(320.), px(150.)), false);
        assert!(editor.drawing.selected.is_none());
    }

    #[::core::prelude::v1::test]
    fn moving_at_double_zoom_uses_document_units() {
        let mut editor = editor();
        editor.drawing.objects.push(Object {
            kind: Kind::Rectangle,
            x: 300.,
            y: 130.,
            w: 140.,
            h: 170.,
            layer: 0,
            keyframe: 1,
            fill: theme::ACCENT,
        });
        editor.zoom = 200;
        editor.pointer_down(point(px(220.), px(75.)), false);
        editor.pointer_move(point(px(260.), px(95.)), false);
        let object = &editor.drawing.objects[0];
        assert_eq!([object.x, object.y], [320., 140.]);
    }

    #[::core::prelude::v1::test]
    fn dropped_image_becomes_selectable_and_copyable_object() {
        let mut editor = editor();
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("popcorn.png");
        let paths = ExternalPaths([path].into_iter().collect());
        editor.import_external_images(&paths, [400., 225.]);
        assert_eq!(editor.drawing.objects.len(), 1);
        assert!(matches!(editor.drawing.objects[0].kind, Kind::Image { .. }));
        assert_eq!(editor.drawing.selected, Some(0));
        editor.apply_edit(Command::Copy);
        editor.apply_edit(Command::Paste);
        assert_eq!(editor.drawing.objects.len(), 2);
        assert!(matches!(editor.drawing.objects[1].kind, Kind::Image { .. }));
    }
}
