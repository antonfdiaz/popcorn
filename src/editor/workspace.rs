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
    #[serde(default)]
    pub(super) rotation: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Corner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

pub(super) enum Drag {
    Draw(usize, [f32; 2]),
    Move {
        items: Vec<(usize, [f32; 2])>,
        start: [f32; 2],
    },
    ScaleGroup {
        items: Vec<TransformItem>,
        corner: Corner,
        bounds: [f32; 4],
        start_size: [f32; 2],
    },
    RotateGroup {
        items: Vec<TransformItem>,
        center: [f32; 2],
        start_pointer_angle: f32,
    },
    Scale {
        index: usize,
        corner: Corner,
        anchor: [f32; 2],
        center: [f32; 2],
        rotation: f32,
        start_size: [f32; 2],
    },
    Rotate {
        index: usize,
        center: [f32; 2],
        start_pointer_angle: f32,
        start_rotation: f32,
    },
    Pan([f32; 2], [f32; 2]),
}

#[derive(Clone, Copy)]
pub(super) struct TransformItem {
    index: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rotation: f32,
}

#[derive(Clone, Copy)]
enum TransformHit {
    Corner(Corner),
    Rotate,
}

pub(super) struct Drawing {
    pub(super) objects: Vec<Object>,
    pub(super) selected: Option<usize>,
    pub(super) selection: Vec<usize>,
    pub(super) drag: Option<Drag>,
    pub(super) pan: [f32; 2],
    pub(super) bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl Default for Drawing {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            selected: None,
            selection: Vec::new(),
            drag: None,
            pan: [0., 0.],
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }
}

impl Editor {
    fn rotated_point(center: [f32; 2], point: [f32; 2], angle: f32) -> [f32; 2] {
        let (sin, cos) = angle.sin_cos();
        let dx = point[0] - center[0];
        let dy = point[1] - center[1];
        [
            center[0] + dx * cos - dy * sin,
            center[1] + dx * sin + dy * cos,
        ]
    }

    fn object_corners(o: &Object) -> [[f32; 2]; 4] {
        let center = [o.x + o.w / 2., o.y + o.h / 2.];
        [
            Self::rotated_point(center, [o.x, o.y], o.rotation),
            Self::rotated_point(center, [o.x + o.w, o.y], o.rotation),
            Self::rotated_point(center, [o.x + o.w, o.y + o.h], o.rotation),
            Self::rotated_point(center, [o.x, o.y + o.h], o.rotation),
        ]
    }

    fn rotated_bounds(o: &Object, scale: f32) -> [f32; 4] {
        let corners = Self::object_corners(o);
        let min_x = corners.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        let max_x = corners
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = corners.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        let max_y = corners
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max);
        [
            min_x * scale,
            min_y * scale,
            (max_x - min_x) * scale,
            (max_y - min_y) * scale,
        ]
    }

    fn local_to_rotated_screen(
        o: &Object,
        local_point: [f32; 2],
        scale: f32,
        bounds: [f32; 4],
    ) -> Point<Pixels> {
        let world = Self::rotated_point(
            [o.x + o.w / 2., o.y + o.h / 2.],
            [o.x + local_point[0], o.y + local_point[1]],
            o.rotation,
        );
        point(
            px(world[0] * scale - bounds[0]),
            px(world[1] * scale - bounds[1]),
        )
    }

    fn render_rotated_vector(o: &Object, scale: f32, bounds: [f32; 4]) -> Div {
        let object = o.clone();
        let fill = theme::color(o.fill);
        let kind = o.kind.clone();
        div()
            .absolute()
            .left(px(bounds[0]))
            .top(px(bounds[1]))
            .w(px(bounds[2].max(1.)))
            .h(px(bounds[3].max(1.)))
            .child(
                canvas(
                    |_, _, _| (),
                    move |canvas_bounds, _, window, _| {
                        let mut path = match &kind {
                            Kind::Pencil(_) => PathBuilder::stroke(px(3. * scale)),
                            _ => PathBuilder::fill(),
                        };
                        match &kind {
                            Kind::Ellipse => {
                                let center = [object.w / 2., object.h / 2.];
                                let mut points = Vec::with_capacity(32);
                                for i in 0..32 {
                                    let t = i as f32 / 32. * std::f32::consts::TAU;
                                    points.push(
                                        canvas_bounds.origin
                                            + Self::local_to_rotated_screen(
                                                &object,
                                                [
                                                    center[0] + center[0] * t.cos(),
                                                    center[1] + center[1] * t.sin(),
                                                ],
                                                scale,
                                                bounds,
                                            ),
                                    );
                                }
                                path.add_polygon(&points, true);
                            }
                            Kind::Pencil(points) => {
                                for (i, p) in points.iter().enumerate() {
                                    let point = canvas_bounds.origin
                                        + Self::local_to_rotated_screen(&object, *p, scale, bounds);
                                    if i == 0 {
                                        path.move_to(point);
                                    } else {
                                        path.line_to(point);
                                    }
                                }
                            }
                            _ => {
                                let points = [
                                    [0., 0.],
                                    [object.w, 0.],
                                    [object.w, object.h],
                                    [0., object.h],
                                ]
                                .map(|point| {
                                    canvas_bounds.origin
                                        + Self::local_to_rotated_screen(
                                            &object, point, scale, bounds,
                                        )
                                });
                                path.add_polygon(&points, true);
                            }
                        }
                        if let Ok(path) = path.build() {
                            window.paint_path(path, fill);
                        }
                    },
                )
                .size_full(),
            )
    }

    fn rotated_image(bytes: &[u8], format: &str, angle: f32) -> Option<Vec<u8>> {
        if angle.abs() < 0.0001 || format == "image/svg+xml" {
            return None;
        }
        let source_format = ::image::ImageFormat::from_mime_type(format)?;
        let source = ::image::load_from_memory_with_format(bytes, source_format)
            .ok()?
            .to_rgba8();
        let (width, height) = source.dimensions();
        let (sin, cos) = angle.sin_cos();
        let new_width = (width as f32 * cos.abs() + height as f32 * sin.abs()).ceil() as u32;
        let new_height = (width as f32 * sin.abs() + height as f32 * cos.abs()).ceil() as u32;
        let mut rotated = ::image::RgbaImage::from_pixel(
            new_width.max(1),
            new_height.max(1),
            ::image::Rgba([0, 0, 0, 0]),
        );
        let source_center = [width as f32 / 2., height as f32 / 2.];
        let output_center = [new_width as f32 / 2., new_height as f32 / 2.];
        for y in 0..new_height {
            for x in 0..new_width {
                let dx = x as f32 - output_center[0];
                let dy = y as f32 - output_center[1];
                let source_x = dx * cos + dy * sin + source_center[0];
                let source_y = -dx * sin + dy * cos + source_center[1];
                if source_x >= 0.
                    && source_y >= 0.
                    && source_x < width as f32
                    && source_y < height as f32
                {
                    rotated.put_pixel(x, y, *source.get_pixel(source_x as u32, source_y as u32));
                }
            }
        }
        let mut output = std::io::Cursor::new(Vec::new());
        ::image::DynamicImage::ImageRgba8(rotated)
            .write_to(&mut output, ::image::ImageFormat::Png)
            .ok()?;
        Some(output.into_inner())
    }

    fn escape_svg_text(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn render_object(o: &Object, scale: f32, selected: bool) -> Div {
        let bounds = Self::rotated_bounds(o, scale);
        if o.rotation.abs() > 0.0001 {
            if matches!(&o.kind, Kind::Rectangle | Kind::Ellipse | Kind::Pencil(_)) {
                return Self::render_rotated_vector(o, scale, bounds).when(selected, |d| {
                    d.border_2().border_color(theme::color(theme::BLUE))
                });
            }
        }
        let base = div()
            .absolute()
            .left(px(bounds[0]))
            .top(px(bounds[1]))
            .w(px(bounds[2].max(1.)))
            .h(px(bounds[3].max(1.)))
            .when(selected, |d| {
                d.border_2().border_color(theme::color(theme::BLUE))
            });
        match &o.kind {
            Kind::Rectangle => base.bg(theme::color(o.fill)),
            Kind::Ellipse => base.rounded_full().bg(theme::color(o.fill)),
            Kind::Text(text) if o.rotation.abs() > 0.0001 => {
                let font_size = (24. * scale * (o.h / 36.)).max(4.);
                let data = format!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}"><text x="0" y="{}" font-family="sans-serif" font-size="{}">{}</text></svg>"#,
                    (o.w * scale).max(1.),
                    (o.h * scale).max(1.),
                    font_size,
                    font_size,
                    Self::escape_svg_text(text),
                );
                div()
                    .absolute()
                    .left(px(o.x * scale))
                    .top(px(o.y * scale))
                    .w(px(o.w * scale))
                    .h(px(o.h * scale))
                    .when(selected, |d| {
                        d.border_2().border_color(theme::color(theme::BLUE))
                    })
                    .child(
                        svg()
                            .data(data.as_bytes())
                            .text_color(theme::color(o.fill))
                            .with_transformation(Transformation::rotate(radians(o.rotation)))
                            .size_full(),
                    )
            }
            Kind::Text(text) => base
                .text_color(theme::color(o.fill))
                .text_size(px((24. * scale * (o.h / 36.)).max(4.)))
                .child(text.clone()),
            Kind::Image { bytes, format } => {
                let image_format = ImageFormat::from_mime_type(format).unwrap_or(ImageFormat::Png);
                let rotated = Self::rotated_image(bytes, format, o.rotation);
                let image_bytes = rotated.clone().unwrap_or_else(|| bytes.clone());
                let image_format = if rotated.is_some() {
                    ImageFormat::Png
                } else {
                    image_format
                };
                base.child(
                    img(ImageSource::Image(Arc::new(Image::from_bytes(
                        image_format,
                        image_bytes,
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
                self.object_visible(o) && !self.layers[o.layer].locked && {
                    let local =
                        Self::rotated_point([o.x + o.w / 2., o.y + o.h / 2.], p, -o.rotation);
                    local[0] >= o.x
                        && local[1] >= o.y
                        && local[0] <= o.x + o.w
                        && local[1] <= o.y + o.h
                }
            })
            .min_by_key(|(i, o)| (o.layer, std::cmp::Reverse(*i)))
            .map(|(i, _)| i)
    }

    pub(super) fn selection_bounds(&self) -> Option<[f32; 4]> {
        let indices = self.selected_indices();
        if indices.is_empty() {
            return None;
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for index in indices {
            let object = &self.drawing.objects[index];
            for point in Self::object_corners(object) {
                min_x = min_x.min(point[0]);
                min_y = min_y.min(point[1]);
                max_x = max_x.max(point[0]);
                max_y = max_y.max(point[1]);
            }
        }
        Some([min_x, min_y, max_x - min_x, max_y - min_y])
    }

    fn selection_transform_hit(&self, p: [f32; 2], scale: f32) -> Option<TransformHit> {
        let [x, y, w, h] = self.selection_bounds()?;
        let tolerance = (10. / scale).max(5.);
        let close = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) <= tolerance;
        let corners = [
            (Corner::TopLeft, [x, y]),
            (Corner::TopRight, [x + w, y]),
            (Corner::BottomRight, [x + w, y + h]),
            (Corner::BottomLeft, [x, y + h]),
        ];
        for (corner, point) in corners {
            if close(point, p) {
                return Some(TransformHit::Corner(corner));
            }
        }
        close([x + w / 2., y - 28.], p).then_some(TransformHit::Rotate)
    }

    fn transform_hit(&self, o: &Object, p: [f32; 2], scale: f32) -> Option<TransformHit> {
        let center = [o.x + o.w / 2., o.y + o.h / 2.];
        let corners = Self::object_corners(o);
        let tolerance = (10. / scale).max(5.);
        let close = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) <= tolerance;
        let corner = [
            Corner::TopLeft,
            Corner::TopRight,
            Corner::BottomRight,
            Corner::BottomLeft,
        ];
        for (corner, point) in corner.into_iter().zip(corners) {
            if close(point, p) {
                return Some(TransformHit::Corner(corner));
            }
        }
        let rotate = Self::rotated_point(center, [center[0], o.y - 28.], o.rotation);
        close(rotate, p).then_some(TransformHit::Rotate)
    }

    fn render_transform_overlay(&self, scale: f32) -> Div {
        let selection = self.selected_indices();
        if selection.is_empty() {
            return div();
        }
        let (corners, rotate, top_mid) = if selection.len() == 1 {
            let object = self.drawing.objects[selection[0]].clone();
            let center = [object.x + object.w / 2., object.y + object.h / 2.];
            let corners = Self::object_corners(&object);
            let top_mid = Self::rotated_point(center, [center[0], object.y], object.rotation);
            (
                corners,
                Self::rotated_point(center, [center[0], object.y - 28.], object.rotation),
                top_mid,
            )
        } else {
            let [x, y, w, h] = self.selection_bounds().unwrap();
            let corners = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
            let center = [x + w / 2., y + h / 2.];
            (corners, [center[0], y - 28.], [center[0], y])
        };
        let handles = corners
            .iter()
            .copied()
            .chain(std::iter::once(rotate))
            .map(|p| {
                div()
                    .absolute()
                    .left(px(p[0] * scale - 5.))
                    .top(px(p[1] * scale - 5.))
                    .w(px(10.))
                    .h(px(10.))
                    .rounded_sm()
                    .bg(theme::color(theme::STAGE))
                    .border_1()
                    .border_color(theme::color(theme::ACCENT))
            });
        let outline = corners;
        div()
            .absolute()
            .inset_0()
            .child(
                canvas(
                    move |_, _, _| (),
                    move |canvas_bounds, _, window, _| {
                        let mut path = PathBuilder::stroke(px(1.));
                        for (i, p) in outline.iter().enumerate() {
                            let p =
                                canvas_bounds.origin + point(px(p[0] * scale), px(p[1] * scale));
                            if i == 0 {
                                path.move_to(p);
                            } else {
                                path.line_to(p);
                            }
                        }
                        path.close();
                        let line_end = canvas_bounds.origin
                            + point(px(rotate[0] * scale), px(rotate[1] * scale));
                        path.move_to(
                            canvas_bounds.origin
                                + point(px(top_mid[0] * scale), px(top_mid[1] * scale)),
                        );
                        path.line_to(line_end);
                        if let Ok(path) = path.build() {
                            window.paint_path(path, theme::color(theme::ACCENT));
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .children(handles)
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
                            .children(
                                objects.into_iter().map(|(i, o)| {
                                    Self::render_object(o, scale, self.is_selected(i))
                                }),
                            )
                            .when(self.selected_tool == Tool::Select, |stage| {
                                stage.child(self.render_transform_overlay(scale))
                            }),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseDownEvent, window, cx| {
                            if let Some(focus) = &this.focus {
                                window.focus(focus, cx);
                            }
                            this.pointer_down_with_modifiers(
                                e.position,
                                e.modifiers.alt,
                                e.modifiers.shift,
                                e.click_count,
                            );
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
                            let hit = this.hit_test(this.document_point(e.position));
                            this.set_selection(hit);
                            if let Some(i) = hit {
                                let layer = this.drawing.objects[i].layer;
                                this.set_layer_selection([layer]);
                                this.clear_timeline_selection();
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
                self.clear_timeline_selection();
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
                rotation: 0.,
            });
            self.set_selection([index]);
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

    #[cfg(test)]
    fn pointer_down(&mut self, pos: Point<Pixels>, alt: bool) {
        self.pointer_down_with_modifiers(pos, alt, false, 1);
    }

    fn pointer_down_with_modifiers(
        &mut self,
        pos: Point<Pixels>,
        alt: bool,
        shift: bool,
        click_count: usize,
    ) {
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
        self.clear_timeline_selection();
        if self.selected_tool == Tool::Select {
            let scale = self.zoom as f32 / 100.;
            let hit = self.hit_test(p);
            if click_count == 2 {
                if let Some(i) = hit {
                    if matches!(self.drawing.objects[i].kind, Kind::Text(_)) {
                        self.set_selection([i]);
                        self.text_editing = Some(i);
                        self.drawing.drag = None;
                        let layer = self.drawing.objects[i].layer;
                        self.set_layer_selection([layer]);
                        return;
                    }
                }
            }

            let selected_before = self.selected_indices();
            let primary_before = self.drawing.selected;
            let preserve_group = selected_before.len() > 1
                && hit.is_some_and(|index| selected_before.contains(&index));
            let handle = if !shift {
                if selected_before.len() == 1 {
                    primary_before
                        .and_then(|i| self.drawing.objects.get(i))
                        .and_then(|o| self.transform_hit(o, p, scale))
                } else if selected_before.len() > 1 {
                    self.selection_transform_hit(p, scale)
                } else {
                    None
                }
            } else {
                None
            };

            if shift {
                if let Some(i) = hit {
                    let mut selection = selected_before;
                    if let Some(position) = selection.iter().position(|selected| *selected == i) {
                        selection.remove(position);
                    } else {
                        selection.push(i);
                    }
                    self.set_selection(selection);
                }
                self.drawing.drag = None;
                return;
            }

            self.set_selection(if handle.is_some() && selected_before.len() > 1 {
                selected_before.clone()
            } else if handle.is_some() {
                primary_before.into_iter().collect()
            } else if preserve_group {
                selected_before.clone()
            } else {
                hit.into_iter().collect()
            });
            self.text_editing = None;
            if let Some(i) = self.drawing.selected {
                self.checkpoint();
                let layer = self.drawing.objects[i].layer;
                self.set_layer_selection([layer]);
                let o = &self.drawing.objects[i];
                let group_transform =
                    selected_before.len() > 1 && (handle.is_some() || preserve_group);
                self.drawing.drag = if group_transform {
                    let items = selected_before
                        .iter()
                        .filter_map(|index| {
                            self.drawing
                                .objects
                                .get(*index)
                                .map(|object| TransformItem {
                                    index: *index,
                                    x: object.x,
                                    y: object.y,
                                    w: object.w,
                                    h: object.h,
                                    rotation: object.rotation,
                                })
                        })
                        .collect::<Vec<_>>();
                    let bounds = self.selection_bounds().unwrap();
                    match handle {
                        Some(TransformHit::Corner(corner)) => Some(Drag::ScaleGroup {
                            items,
                            corner,
                            bounds,
                            start_size: [bounds[2], bounds[3]],
                        }),
                        Some(TransformHit::Rotate) => {
                            let center = [bounds[0] + bounds[2] / 2., bounds[1] + bounds[3] / 2.];
                            Some(Drag::RotateGroup {
                                items,
                                center,
                                start_pointer_angle: (p[1] - center[1]).atan2(p[0] - center[0]),
                            })
                        }
                        None if preserve_group => Some(Drag::Move {
                            items: items
                                .iter()
                                .map(|item| (item.index, [item.x, item.y]))
                                .collect(),
                            start: p,
                        }),
                        None => None,
                    }
                } else if let Some(handle) = handle {
                    let center = [o.x + o.w / 2., o.y + o.h / 2.];
                    match handle {
                        TransformHit::Rotate => Some(Drag::Rotate {
                            index: i,
                            center,
                            start_pointer_angle: (p[1] - center[1]).atan2(p[0] - center[0]),
                            start_rotation: o.rotation,
                        }),
                        TransformHit::Corner(corner) => {
                            let anchor = match corner {
                                Corner::TopLeft => [o.x + o.w, o.y + o.h],
                                Corner::TopRight => [o.x, o.y + o.h],
                                Corner::BottomRight => [o.x, o.y],
                                Corner::BottomLeft => [o.x + o.w, o.y],
                            };
                            Some(Drag::Scale {
                                index: i,
                                corner,
                                anchor,
                                center,
                                rotation: o.rotation,
                                start_size: [o.w, o.h],
                            })
                        }
                    }
                } else if hit.is_some() {
                    let items = self
                        .selected_indices()
                        .into_iter()
                        .filter_map(|index| {
                            self.drawing
                                .objects
                                .get(index)
                                .map(|object| (index, [object.x, object.y]))
                        })
                        .collect();
                    Some(Drag::Move { items, start: p })
                } else {
                    None
                };
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
            rotation: 0.,
        });
        self.set_selection([i]);
        self.text_editing = text.then_some(i);
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
            Some(Drag::ScaleGroup {
                items,
                corner,
                bounds,
                start_size,
            }) => {
                let [x, y, width, height] = *bounds;
                let [anchor_x, anchor_y] = match corner {
                    Corner::TopLeft => [x + width, y + height],
                    Corner::TopRight => [x, y + height],
                    Corner::BottomRight => [x, y],
                    Corner::BottomLeft => [x + width, y],
                };
                let mut new_width = match corner {
                    Corner::TopLeft | Corner::BottomLeft => anchor_x - p[0],
                    Corner::TopRight | Corner::BottomRight => p[0] - anchor_x,
                }
                .abs()
                .max(2.);
                let mut new_height = match corner {
                    Corner::TopLeft | Corner::TopRight => anchor_y - p[1],
                    Corner::BottomLeft | Corner::BottomRight => p[1] - anchor_y,
                }
                .abs()
                .max(2.);
                if shift {
                    let ratio = start_size[0] / start_size[1];
                    if new_width / new_height > ratio {
                        new_height = (new_width / ratio).max(2.);
                    } else {
                        new_width = (new_height * ratio).max(2.);
                    }
                }
                let new_min = match corner {
                    Corner::TopLeft => [anchor_x - new_width, anchor_y - new_height],
                    Corner::TopRight => [anchor_x, anchor_y - new_height],
                    Corner::BottomRight => [anchor_x, anchor_y],
                    Corner::BottomLeft => [anchor_x - new_width, anchor_y],
                };
                let scale_x = new_width / width.max(0.001);
                let scale_y = new_height / height.max(0.001);
                for item in items {
                    let object = &mut self.drawing.objects[item.index];
                    object.x = new_min[0] + (item.x - x) * scale_x;
                    object.y = new_min[1] + (item.y - y) * scale_y;
                    object.w = (item.w * scale_x).max(2.);
                    object.h = (item.h * scale_y).max(2.);
                }
            }
            Some(Drag::RotateGroup {
                items,
                center,
                start_pointer_angle,
            }) => {
                let mut delta = (p[1] - center[1]).atan2(p[0] - center[0]) - *start_pointer_angle;
                if shift {
                    let increment = 15_f32.to_radians();
                    delta = (delta / increment).round() * increment;
                }
                for item in items {
                    let item_center = [item.x + item.w / 2., item.y + item.h / 2.];
                    let next_center = Self::rotated_point(*center, item_center, delta);
                    let object = &mut self.drawing.objects[item.index];
                    object.x = next_center[0] - item.w / 2.;
                    object.y = next_center[1] - item.h / 2.;
                    object.rotation = item.rotation + delta;
                }
            }
            Some(Drag::Move { items, start }) => {
                let delta = [p[0] - start[0], p[1] - start[1]];
                for (index, initial) in items {
                    let o = &mut self.drawing.objects[*index];
                    o.x = initial[0] + delta[0];
                    o.y = initial[1] + delta[1];
                }
            }
            Some(Drag::Scale {
                index,
                corner,
                anchor,
                center,
                rotation,
                start_size,
            }) => {
                let local = Self::rotated_point(*center, p, -*rotation);
                let mut width = (local[0] - anchor[0]).abs();
                let mut height = (local[1] - anchor[1]).abs();
                width = width.max(2.);
                height = height.max(2.);
                if shift {
                    let ratio = start_size[0] / start_size[1];
                    if width / height > ratio {
                        height = (width / ratio).max(2.);
                    } else {
                        width = (height * ratio).max(2.);
                    }
                }
                let sign_x = match corner {
                    Corner::TopLeft | Corner::BottomLeft => -1.,
                    Corner::TopRight | Corner::BottomRight => 1.,
                };
                let sign_y = match corner {
                    Corner::TopLeft | Corner::TopRight => -1.,
                    Corner::BottomLeft | Corner::BottomRight => 1.,
                };
                let handle_local = [anchor[0] + sign_x * width, anchor[1] + sign_y * height];
                let next_center_local = [
                    (anchor[0] + handle_local[0]) / 2.,
                    (anchor[1] + handle_local[1]) / 2.,
                ];
                let next_center = Self::rotated_point(*center, next_center_local, *rotation);
                let o = &mut self.drawing.objects[*index];
                o.w = width;
                o.h = height;
                o.x = next_center[0] - width / 2.;
                o.y = next_center[1] - height / 2.;
            }
            Some(Drag::Rotate {
                index,
                center,
                start_pointer_angle,
                start_rotation,
            }) => {
                let mut rotation = *start_rotation + (p[1] - center[1]).atan2(p[0] - center[0])
                    - *start_pointer_angle;
                if shift {
                    let increment = 15_f32.to_radians();
                    rotation = (rotation / increment).round() * increment;
                }
                self.drawing.objects[*index].rotation = rotation;
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

    pub(super) fn cancel_interaction(&mut self) {
        if let Some(drag) = self.drawing.drag.take() {
            match drag {
                Drag::Draw(index, _) => {
                    if index < self.drawing.objects.len() {
                        self.drawing.objects.remove(index);
                    }
                    self.clear_selection();
                }
                Drag::Move { items, .. } => {
                    for (index, initial) in items {
                        if let Some(object) = self.drawing.objects.get_mut(index) {
                            object.x = initial[0];
                            object.y = initial[1];
                        }
                    }
                }
                Drag::ScaleGroup { items, .. } | Drag::RotateGroup { items, .. } => {
                    for item in items {
                        if let Some(object) = self.drawing.objects.get_mut(item.index) {
                            object.x = item.x;
                            object.y = item.y;
                            object.w = item.w;
                            object.h = item.h;
                            object.rotation = item.rotation;
                        }
                    }
                }
                Drag::Scale {
                    index,
                    center,
                    rotation,
                    start_size,
                    ..
                } => {
                    if let Some(object) = self.drawing.objects.get_mut(index) {
                        object.w = start_size[0];
                        object.h = start_size[1];
                        object.x = center[0] - start_size[0] / 2.;
                        object.y = center[1] - start_size[1] / 2.;
                        object.rotation = rotation;
                    }
                }
                Drag::Rotate {
                    index,
                    start_rotation,
                    ..
                } => {
                    if let Some(object) = self.drawing.objects.get_mut(index) {
                        object.rotation = start_rotation;
                    }
                }
                Drag::Pan(_, initial) => self.drawing.pan = initial,
            }
        }

        if let Some(index) = self.text_editing.take() {
            let remove =
                self.drawing.objects.get(index).is_some_and(
                    |object| matches!(&object.kind, Kind::Text(text) if text.is_empty()),
                );
            if remove {
                self.drawing.objects.remove(index);
                self.clear_selection();
            }
        }
    }

    pub(super) fn edit_text(&mut self, event: &KeyDownEvent) -> bool {
        if self.modal.is_some()
            || event.keystroke.modifiers.platform
            || event.keystroke.modifiers.control
            || event.keystroke.modifiers.alt
        {
            return false;
        }
        let i = self.text_editing.or_else(|| {
            (self.selected_tool == Tool::Text)
                .then(|| self.editable_selection())
                .flatten()
        });
        let Some(i) = i else {
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
            rotation: 0.,
        });
        editor.zoom = 200;
        editor.pointer_down(point(px(220.), px(75.)), false);
        editor.pointer_move(point(px(260.), px(95.)), false);
        let object = &editor.drawing.objects[0];
        assert_eq!([object.x, object.y], [320., 140.]);
    }

    #[::core::prelude::v1::test]
    fn shift_selects_multiple_objects_and_moves_them_as_a_group() {
        let mut editor = editor();
        editor.drawing.objects.extend([
            Object {
                kind: Kind::Rectangle,
                x: 100.,
                y: 100.,
                w: 40.,
                h: 40.,
                layer: 0,
                keyframe: 1,
                fill: theme::ACCENT,
                rotation: 0.,
            },
            Object {
                kind: Kind::Rectangle,
                x: 200.,
                y: 100.,
                w: 40.,
                h: 40.,
                layer: 0,
                keyframe: 1,
                fill: theme::BLUE,
                rotation: 0.,
            },
        ]);
        editor.selected_tool = Tool::Select;
        editor.pointer_down(point(px(120.), px(120.)), false);
        editor.pointer_down_with_modifiers(point(px(220.), px(120.)), false, true, 1);
        assert_eq!(editor.selected_indices(), vec![0, 1]);
        editor.pointer_down(point(px(120.), px(120.)), false);
        editor.pointer_move(point(px(140.), px(150.)), false);
        assert_eq!(
            [editor.drawing.objects[0].x, editor.drawing.objects[0].y],
            [120., 130.]
        );
        assert_eq!(
            [editor.drawing.objects[1].x, editor.drawing.objects[1].y],
            [220., 130.]
        );
    }

    #[::core::prelude::v1::test]
    fn group_transform_scales_and_rotates_all_selected_objects() {
        let mut editor = editor();
        editor.drawing.objects.extend([
            Object {
                kind: Kind::Rectangle,
                x: 100.,
                y: 100.,
                w: 40.,
                h: 40.,
                layer: 0,
                keyframe: 1,
                fill: theme::ACCENT,
                rotation: 0.,
            },
            Object {
                kind: Kind::Rectangle,
                x: 200.,
                y: 100.,
                w: 40.,
                h: 40.,
                layer: 0,
                keyframe: 1,
                fill: theme::BLUE,
                rotation: 0.,
            },
        ]);
        editor.selected_tool = Tool::Select;
        editor.set_selection([0, 1]);
        editor.pointer_down_with_modifiers(point(px(240.), px(140.)), false, false, 1);
        editor.pointer_move(point(px(380.), px(220.)), false);
        assert_eq!(
            [editor.drawing.objects[0].x, editor.drawing.objects[0].y],
            [100., 100.]
        );
        assert_eq!(
            [editor.drawing.objects[0].w, editor.drawing.objects[0].h],
            [80., 120.]
        );
        assert_eq!(
            [editor.drawing.objects[1].x, editor.drawing.objects[1].y],
            [300., 100.]
        );

        editor.drawing.drag = None;
        editor.pointer_down_with_modifiers(point(px(240.), px(72.)), false, false, 1);
        editor.pointer_move(point(px(288.), px(160.)), false);
        assert!((editor.drawing.objects[0].rotation - std::f32::consts::FRAC_PI_2).abs() < 0.01);
        assert!((editor.drawing.objects[1].rotation - std::f32::consts::FRAC_PI_2).abs() < 0.01);
    }

    #[::core::prelude::v1::test]
    fn escape_cancels_a_new_shape_and_double_click_enters_text_editing() {
        let mut editor = editor();
        editor.selected_tool = Tool::Rectangle;
        editor.pointer_down(point(px(100.), px(100.)), false);
        editor.pointer_move(point(px(180.), px(160.)), false);
        editor.cancel_interaction();
        assert!(editor.drawing.objects.is_empty());
        assert!(editor.drawing.drag.is_none());

        editor.drawing.objects.push(Object {
            kind: Kind::Text("Hello".into()),
            x: 100.,
            y: 100.,
            w: 200.,
            h: 36.,
            layer: 0,
            keyframe: 1,
            fill: theme::TEXT,
            rotation: 0.,
        });
        editor.selected_tool = Tool::Select;
        editor.pointer_down_with_modifiers(point(px(120.), px(120.)), false, false, 2);
        assert_eq!(editor.text_editing, Some(0));
        assert_eq!(editor.drawing.selected, Some(0));
    }

    #[::core::prelude::v1::test]
    fn selection_controls_scale_from_the_opposite_corner() {
        let mut editor = editor();
        editor.drawing.objects.push(Object {
            kind: Kind::Rectangle,
            x: 300.,
            y: 100.,
            w: 100.,
            h: 80.,
            layer: 0,
            keyframe: 1,
            fill: theme::ACCENT,
            rotation: 0.,
        });
        editor.selected_tool = Tool::Select;
        editor.pointer_down(point(px(350.), px(140.)), false);
        editor.drawing.drag = None;
        editor.pointer_down(point(px(400.), px(180.)), false);
        editor.pointer_move(point(px(500.), px(220.)), false);
        let object = &editor.drawing.objects[0];
        assert_eq!(
            [object.x, object.y, object.w, object.h],
            [300., 100., 200., 120.]
        );
    }

    #[::core::prelude::v1::test]
    fn transform_rotate_handle_updates_object_rotation() {
        let mut editor = editor();
        editor.drawing.objects.push(Object {
            kind: Kind::Rectangle,
            x: 300.,
            y: 100.,
            w: 100.,
            h: 80.,
            layer: 0,
            keyframe: 1,
            fill: theme::ACCENT,
            rotation: 0.,
        });
        editor.selected_tool = Tool::Select;
        editor.pointer_down(point(px(350.), px(140.)), false);
        editor.drawing.drag = None;
        editor.pointer_down(point(px(350.), px(72.)), false);
        editor.pointer_move(point(px(418.), px(140.)), false);
        assert!((editor.drawing.objects[0].rotation - std::f32::consts::FRAC_PI_2).abs() < 0.01);
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
