//! Boolean vector editing. Persist normalized geometry so ordinary object transforms
//! apply equally to cut-out shapes, disconnected fragments, and their holes.
use super::*;
use geo::{
    Area, BooleanOps, BoundingRect, Buffer, Contains, Coord, Intersects, LineString, MapCoords,
    MultiPolygon, Point as GeoPoint, Polygon, TriangulateEarcut, Validation,
};
use workspace::{Drag, FillRegion, Kind, Object};

#[cfg(test)]
mod tests;

impl Object {
    pub(super) fn is_stroke(&self) -> bool {
        matches!(
            self.kind,
            Kind::Pencil(_) | Kind::Vector { stroke: true, .. }
        )
    }

    pub(super) fn vector_geometry(&self) -> Option<MultiPolygon<f64>> {
        let local = match &self.kind {
            Kind::Rectangle => MultiPolygon(vec![Polygon::new(
                LineString::from(vec![
                    (0., 0.),
                    (self.w as f64, 0.),
                    (self.w as f64, self.h as f64),
                    (0., self.h as f64),
                    (0., 0.),
                ]),
                vec![],
            )]),
            Kind::Ellipse => {
                let points = (0..=128)
                    .map(|i| {
                        let angle = i as f64 * std::f64::consts::TAU / 128.;
                        (
                            self.w as f64 * (1. + angle.cos()) / 2.,
                            self.h as f64 * (1. + angle.sin()) / 2.,
                        )
                    })
                    .collect::<Vec<_>>();
                MultiPolygon(vec![Polygon::new(LineString::from(points), vec![])])
            }
            Kind::Pencil(points) => {
                let line = LineString::from(
                    points
                        .iter()
                        .map(|p| (p[0] as f64, p[1] as f64))
                        .collect::<Vec<_>>(),
                );
                if points.len() == 1 {
                    GeoPoint::new(points[0][0] as f64, points[0][1] as f64).buffer(1.5)
                } else {
                    line.buffer(1.5)
                }
            }
            Kind::Vector { geometry, .. } => geometry.map_coords(|p| Coord {
                x: p.x * self.w as f64,
                y: p.y * self.h as f64,
            }),
            Kind::Text(_) | Kind::Image { .. } => return None,
        };
        let (sin, cos) = (self.rotation as f64).sin_cos();
        let cx = self.w as f64 / 2.;
        let cy = self.h as f64 / 2.;
        Some(local.map_coords(|p| Coord {
            x: self.x as f64 + cx + (p.x - cx) * cos - (p.y - cy) * sin,
            y: self.y as f64 + cy + (p.x - cx) * sin + (p.y - cy) * cos,
        }))
    }

    pub(super) fn fill_geometries(&self) -> Vec<(MultiPolygon<f64>, u32)> {
        let Kind::Vector { fills, .. } = &self.kind else {
            return Vec::new();
        };
        let (sin, cos) = (self.rotation as f64).sin_cos();
        let cx = self.w as f64 / 2.;
        let cy = self.h as f64 / 2.;
        fills
            .iter()
            .map(|fill| {
                (
                    fill.geometry.map_coords(|p| {
                        let x = p.x * self.w as f64;
                        let y = p.y * self.h as f64;
                        Coord {
                            x: self.x as f64 + cx + (x - cx) * cos - (y - cy) * sin,
                            y: self.y as f64 + cy + (x - cx) * sin + (y - cy) * cos,
                        }
                    }),
                    fill.color,
                )
            })
            .collect()
    }

    pub(super) fn with_geometry(&self, geometry: MultiPolygon<f64>, stroke: bool) -> Option<Self> {
        self.with_geometry_and_fills(geometry, Vec::new(), stroke)
    }

    pub(super) fn with_geometry_and_fills(
        &self,
        geometry: MultiPolygon<f64>,
        fills: Vec<(MultiPolygon<f64>, u32)>,
        stroke: bool,
    ) -> Option<Self> {
        let bounds = geometry.bounding_rect()?;
        let x = bounds.min().x;
        let y = bounds.min().y;
        let w = bounds.width().max(0.001);
        let h = bounds.height().max(0.001);
        Some(Self {
            kind: Kind::Vector {
                geometry: geometry.map_coords(|p| Coord {
                    x: (p.x - x) / w,
                    y: (p.y - y) / h,
                }),
                stroke,
                fills: fills
                    .into_iter()
                    .map(|(geometry, color)| FillRegion {
                        geometry: geometry.map_coords(|p| Coord {
                            x: (p.x - x) / w,
                            y: (p.y - y) / h,
                        }),
                        color,
                    })
                    .collect(),
            },
            x: x as f32,
            y: y as f32,
            w: w as f32,
            h: h as f32,
            rotation: 0.,
            ..self.clone()
        })
    }

    pub(super) fn valid_geometry(&self) -> bool {
        match &self.kind {
            Kind::Vector {
                geometry, fills, ..
            } => {
                !geometry.0.is_empty()
                    && geometry.is_valid()
                    && fills.iter().all(|fill| fill.geometry.is_valid())
            }
            Kind::Pencil(points) => points.iter().flatten().all(|n| n.is_finite()),
            _ => true,
        }
    }
}

impl Editor {
    fn editable_paint_layer(&self) -> bool {
        !self.layers[self.selected_layer].locked && self.layers[self.selected_layer].visible
    }

    pub(super) fn erase_segment(&mut self, from: [f32; 2], to: [f32; 2]) {
        if !self.editable_paint_layer() {
            return;
        }
        let radius = self.eraser_size as f64 / 2.;
        let brush = if from == to {
            GeoPoint::new(to[0] as f64, to[1] as f64).buffer(radius)
        } else {
            LineString::from(vec![
                (from[0] as f64, from[1] as f64),
                (to[0] as f64, to[1] as f64),
            ])
            .buffer(radius)
        };
        // Clip the swept brush to the stage; geometry outside the stage is untouched.
        let stage = geo::Rect::new(
            Coord { x: 0., y: 0. },
            Coord {
                x: self.stage_size[0] as f64,
                y: self.stage_size[1] as f64,
            },
        )
        .to_polygon();
        let brush = brush.intersection(&stage);
        let mut replacements = Vec::new();
        for (i, object) in self.drawing.objects.iter().enumerate() {
            if object.layer != self.selected_layer || !self.object_visible(object) {
                continue;
            }
            let Some(shape) = object.vector_geometry() else {
                continue;
            };
            if !shape.intersects(&brush) {
                continue;
            }
            let remaining = shape.difference(&brush);
            let fills = object
                .fill_geometries()
                .into_iter()
                .filter_map(|(fill, color)| {
                    let fill = fill.difference(&brush);
                    (!fill.0.is_empty()).then_some((fill, color))
                })
                .collect();
            if shape.unsigned_area() - remaining.unsigned_area() < 0.000001 {
                continue;
            }
            replacements.push((
                i,
                object.with_geometry_and_fills(remaining, fills, object.is_stroke()),
            ));
        }
        if replacements.is_empty() {
            return;
        }
        if !matches!(self.drawing.drag, Some(Drag::Erase { changed: true, .. })) {
            self.checkpoint();
        }
        for (i, replacement) in replacements.into_iter().rev() {
            if let Some(object) = replacement {
                self.drawing.objects[i] = object;
            } else {
                self.drawing.objects.remove(i);
            }
        }
        self.clear_selection();
        if let Some(Drag::Erase { changed, .. }) = &mut self.drawing.drag {
            *changed = true;
        }
    }

    pub(super) fn fill_region(&mut self, p: [f32; 2]) {
        if !self.editable_paint_layer()
            || p[0] < 0.
            || p[1] < 0.
            || p[0] > self.stage_size[0]
            || p[1] > self.stage_size[1]
        {
            return;
        }
        let point = GeoPoint::new(p[0] as f64, p[1] as f64);
        let shapes = self
            .drawing
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.layer == self.selected_layer && self.object_visible(o))
            .filter_map(|(i, o)| o.vector_geometry().map(|g| (i, o.is_stroke(), g)))
            .collect::<Vec<_>>();
        let mut strokes = MultiPolygon::empty();
        let mut occupied = MultiPolygon::empty();
        for (_, stroke, geometry) in &shapes {
            occupied = occupied.union(geometry);
            if *stroke {
                strokes = strokes.union(geometry);
            }
        }
        for (index, _, _) in &shapes {
            for (fill_index, (geometry, color)) in self.drawing.objects[*index]
                .fill_geometries()
                .into_iter()
                .enumerate()
            {
                if geometry.contains(&point) {
                    if color == self.fill {
                        return;
                    }
                    self.checkpoint();
                    if let Kind::Vector { fills, .. } = &mut self.drawing.objects[*index].kind {
                        if let Some(fill) = fills.get_mut(fill_index) {
                            fill.color = self.fill;
                        }
                    }
                    self.clear_selection();
                    return;
                }
            }
        }
        // Paint existing fill components without recoloring the linework. A stroke
        // crossing a filled shape partitions that fill into independently paintable regions.
        if strokes.contains(&point) {
            return;
        }
        for (index, stroke, geometry) in shapes.iter().rev() {
            if *stroke || !geometry.contains(&point) {
                continue;
            }
            let regions = if strokes.0.is_empty() {
                geometry.clone()
            } else {
                geometry.difference(&strokes)
            };
            let Some(region) = regions.0.into_iter().find(|region| region.contains(&point)) else {
                continue;
            };
            let object = self.drawing.objects[*index].clone();
            if strokes.0.is_empty() && object.fill == self.fill {
                return;
            }
            self.checkpoint();
            if strokes.0.is_empty() {
                self.drawing.objects[*index].fill = self.fill;
            } else {
                let region = MultiPolygon(vec![region]);
                let remaining = geometry.difference(&region);
                let mut fills = object.fill_geometries();
                fills.push((region, self.fill));
                self.drawing.objects[*index] = object
                    .with_geometry_and_fills(remaining, fills, object.is_stroke())
                    .expect("nonempty filled shape");
            }
            self.clear_selection();
            return;
        }
        // Holes in the union are bounded regions, including loops made from several
        // crossing strokes. Subtract occupied islands to preserve nested drawings.
        let mut candidates = Vec::new();
        for polygon in &occupied.0 {
            for ring in polygon.interiors() {
                let hole = Polygon::new(ring.clone(), vec![]);
                if hole.contains(&point) {
                    for region in hole.difference(&occupied).0 {
                        if region.contains(&point) {
                            candidates.push(region);
                        }
                    }
                }
            }
        }
        let Some(region) = candidates
            .into_iter()
            .min_by(|a, b| a.unsigned_area().total_cmp(&b.unsigned_area()))
        else {
            self.status = "Fill needs a closed region on the active layer".into();
            return;
        };
        let stroke_indices = shapes
            .iter()
            .filter(|(_, stroke, _)| *stroke)
            .map(|(i, _, _)| *i)
            .collect::<Vec<_>>();
        let Some(&host_index) = stroke_indices.first() else {
            return;
        };
        let host = self.drawing.objects[host_index].clone();
        let mut fills = host.fill_geometries();
        fills.push((MultiPolygon(vec![region]), self.fill));
        let object = host
            .with_geometry_and_fills(strokes, fills, true)
            .expect("nonempty closed region");
        self.checkpoint();
        self.drawing.objects[host_index] = object;
        for index in stroke_indices
            .into_iter()
            .filter(|index| *index != host_index)
            .rev()
        {
            self.drawing.objects.remove(index);
        }
        self.clear_selection();
    }

    pub(super) fn render_vector_shape(o: &Object, scale: f32, selected: bool) -> Div {
        let bounds = Self::rotated_bounds(o, scale);
        let geometry = o.vector_geometry().expect("vector object");
        // Triangulation preserves holes regardless of winding and keeps erased
        // regions transparent (rather than painting them with the stage color).
        let triangles = geometry
            .0
            .iter()
            .flat_map(|p| p.earcut_triangles())
            .collect::<Vec<_>>();
        let fill = o.fill;
        let fill_regions = o
            .fill_geometries()
            .into_iter()
            .map(|(geometry, color)| {
                (
                    geometry
                        .0
                        .iter()
                        .flat_map(|p| p.earcut_triangles())
                        .collect::<Vec<_>>(),
                    color,
                )
            })
            .collect::<Vec<_>>();
        div()
            .absolute()
            .left(px(bounds[0]))
            .top(px(bounds[1]))
            .w(px(bounds[2].max(1.)))
            .h(px(bounds[3].max(1.)))
            .when(selected, |d| {
                d.border_2().border_color(theme::color(theme::BLUE))
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |canvas_bounds, _, window, _| {
                        let mut path = PathBuilder::fill();
                        for triangle in &triangles {
                            let points = triangle.to_array().map(|p| {
                                canvas_bounds.origin
                                    + point(
                                        px(p.x as f32 * scale - bounds[0]),
                                        px(p.y as f32 * scale - bounds[1]),
                                    )
                            });
                            path.add_polygon(&points, true);
                        }
                        if let Ok(path) = path.build() {
                            window.paint_path(path, theme::color(fill));
                        }
                        for (triangles, color) in &fill_regions {
                            let mut path = PathBuilder::fill();
                            for triangle in triangles {
                                let points = triangle.to_array().map(|p| {
                                    canvas_bounds.origin
                                        + point(
                                            px(p.x as f32 * scale - bounds[0]),
                                            px(p.y as f32 * scale - bounds[1]),
                                        )
                                });
                                path.add_polygon(&points, true);
                            }
                            if let Ok(path) = path.build() {
                                window.paint_path(path, theme::color(*color));
                            }
                        }
                    },
                )
                .size_full(),
            )
    }
}
