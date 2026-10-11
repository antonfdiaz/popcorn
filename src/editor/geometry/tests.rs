use super::{Command, Editor, Kind, LayerState, Object, commands, theme};
use geo::{Contains, Point as GeoPoint};

fn rectangle() -> Object {
    Object {
        kind: Kind::Rectangle,
        x: 100.,
        y: 100.,
        w: 100.,
        h: 100.,
        layer: 0,
        keyframe: 1,
        fill: theme::BLUE,
        rotation: 0.,
    }
}

fn stroke(points: &[[f32; 2]]) -> Object {
    Object {
        kind: Kind::Pencil(points.to_vec()),
        x: 0.,
        y: 0.,
        w: 300.,
        h: 300.,
        ..rectangle()
    }
}

fn contains(object: &Object, p: [f64; 2]) -> bool {
    object
        .vector_geometry()
        .unwrap()
        .contains(&GeoPoint::new(p[0], p[1]))
}

#[test]
fn eraser_cuts_a_transparent_hole_and_preserves_the_surrounding_fill() {
    let mut editor = Editor::default();
    editor.drawing.objects.push(rectangle());
    editor.erase_segment([150., 150.], [150., 150.]);
    let object = &editor.drawing.objects[0];
    assert!(!contains(object, [150., 150.]));
    assert!(contains(object, [120., 120.]));
    assert_eq!(object.fill, theme::BLUE);
    assert!(object.valid_geometry());
    assert_eq!(object.vector_geometry().unwrap().0[0].interiors().len(), 1);
}

#[test]
fn a_fast_eraser_sweep_splits_a_stroke_without_deleting_its_ends() {
    let mut editor = Editor::default();
    editor
        .drawing
        .objects
        .push(stroke(&[[100., 150.], [200., 150.]]));
    editor.erase_segment([150., 50.], [150., 250.]);
    let object = &editor.drawing.objects[0];
    assert!(object.is_stroke());
    assert!(contains(object, [110., 150.]));
    assert!(contains(object, [190., 150.]));
    assert!(!contains(object, [150., 150.]));
    assert_eq!(object.vector_geometry().unwrap().0.len(), 2);
    assert!(object.valid_geometry());
}

#[test]
fn erasing_only_edits_the_active_visible_keyframe_and_preserves_nonvector_objects() {
    let mut editor = Editor::default();
    editor.layers.push(LayerState::new("Other", theme::BLUE));
    editor.layers[0].keyframes.push(8);
    editor.drawing.objects.extend([
        rectangle(),
        Object {
            layer: 1,
            ..rectangle()
        },
        Object {
            keyframe: 8,
            ..rectangle()
        },
        Object {
            kind: Kind::Text("Text".into()),
            ..rectangle()
        },
        Object {
            kind: Kind::Image {
                bytes: vec![],
                format: "image/png".into(),
            },
            ..rectangle()
        },
    ]);
    editor.erase_segment([150., 150.], [150., 150.]);
    assert!(matches!(
        editor.drawing.objects[0].kind,
        Kind::Vector { .. }
    ));
    assert!(matches!(editor.drawing.objects[1].kind, Kind::Rectangle));
    assert!(matches!(editor.drawing.objects[2].kind, Kind::Rectangle));
    assert!(matches!(editor.drawing.objects[3].kind, Kind::Text(_)));
    assert!(matches!(editor.drawing.objects[4].kind, Kind::Image { .. }));
    let saved = serde_json::to_string(&editor.snapshot()).unwrap();
    editor.layers[0].visible = false;
    editor.erase_segment([120., 120.], [120., 120.]);
    editor.layers[0].visible = true;
    assert_eq!(serde_json::to_string(&editor.snapshot()).unwrap(), saved);
}

fn box_strokes() -> Vec<Object> {
    vec![
        stroke(&[[100., 100.], [200., 100.]]),
        stroke(&[[200., 100.], [200., 200.]]),
        stroke(&[[200., 200.], [100., 200.]]),
        stroke(&[[100., 200.], [100., 100.]]),
    ]
}

#[test]
fn fill_detects_a_region_bounded_by_multiple_strokes_and_keeps_their_color() {
    let mut editor = Editor::default();
    editor.drawing.objects = box_strokes();
    editor.fill_region([150., 150.]);
    assert_eq!(editor.drawing.objects.len(), 1);
    assert!(editor.drawing.objects[0].is_stroke());
    assert!(
        editor.drawing.objects[0]
            .fill_geometries()
            .iter()
            .any(|(geometry, color)| *color == editor.fill
                && geometry.contains(&GeoPoint::new(150., 150.)))
    );
    assert_eq!(editor.drawing.objects[0].fill, theme::BLUE);
    assert_eq!(editor.drawing.objects[0].fill_geometries().len(), 1);
    let state = serde_json::to_string(&editor.snapshot()).unwrap();
    editor.fill_region([150., 150.]);
    assert_eq!(serde_json::to_string(&editor.snapshot()).unwrap(), state);
    assert_eq!(editor.history.len(), 1);
    editor.apply_edit(Command::Undo);
    assert_eq!(editor.drawing.objects.len(), 4);
    editor.apply_edit(Command::Redo);
    assert_eq!(serde_json::to_string(&editor.snapshot()).unwrap(), state);
}

#[test]
fn open_regions_and_clicks_on_strokes_do_not_fill_or_create_undo_entries() {
    let mut editor = Editor::default();
    editor.drawing.objects = box_strokes();
    editor.drawing.objects.pop();
    editor.fill_region([150., 150.]);
    editor.fill_region([150., 100.]);
    assert_eq!(editor.drawing.objects.len(), 3);
    assert!(editor.history.is_empty());
}

#[test]
fn crossing_strokes_partition_regions_and_preserve_nested_islands() {
    let mut editor = Editor::default();
    editor.drawing.objects = box_strokes();
    editor
        .drawing
        .objects
        .push(stroke(&[[150., 100.], [150., 200.]]));
    editor.drawing.objects.push(Object {
        x: 110.,
        y: 110.,
        w: 20.,
        h: 20.,
        ..rectangle()
    });
    editor.fill_region([140., 150.]);
    let fill = &editor.drawing.objects[0];
    assert!(
        fill.fill_geometries()
            .iter()
            .any(|(geometry, _)| geometry.contains(&GeoPoint::new(140., 150.)))
    );
    assert!(
        !fill
            .fill_geometries()
            .iter()
            .any(|(geometry, _)| geometry.contains(&GeoPoint::new(170., 150.)))
    );
    assert!(
        !fill
            .fill_geometries()
            .iter()
            .any(|(geometry, _)| geometry.contains(&GeoPoint::new(120., 120.)))
    );
    assert!(fill.valid_geometry());
    assert_eq!(fill.fill_geometries().len(), 1);
}

#[test]
fn repainting_one_side_of_a_divided_fill_leaves_the_other_side_and_stroke_untouched() {
    let mut editor = Editor::default();
    editor.drawing.objects = vec![rectangle(), stroke(&[[150., 100.], [150., 200.]])];
    editor.fill_region([125., 150.]);
    assert_eq!(editor.drawing.objects.len(), 2);
    assert_eq!(editor.drawing.objects[0].fill, theme::BLUE);
    assert!(contains(&editor.drawing.objects[0], [175., 150.]));
    assert!(!contains(&editor.drawing.objects[0], [125., 150.]));
    assert!(
        editor.drawing.objects[0]
            .fill_geometries()
            .iter()
            .any(|(geometry, color)| *color == editor.fill
                && geometry.contains(&GeoPoint::new(125., 150.)))
    );
    assert!(editor.drawing.objects[1].is_stroke());
    assert_eq!(editor.drawing.objects[1].fill, theme::BLUE);
}

#[test]
fn erased_geometry_and_holes_survive_document_round_trip_and_transforms() {
    let mut editor = Editor::default();
    editor.drawing.objects.push(rectangle());
    editor.erase_segment([150., 150.], [150., 150.]);
    let json = serde_json::to_string(&editor.snapshot()).unwrap();
    let snapshot: commands::Snapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&snapshot).unwrap(), json);
    let mut object = editor.drawing.objects[0].clone();
    object.w *= 2.;
    object.h *= 2.;
    object.rotation = std::f32::consts::FRAC_PI_2;
    assert!(!contains(&object, [200., 200.]));
    assert!(contains(&object, [250., 150.]));
    assert!(object.valid_geometry());
}
