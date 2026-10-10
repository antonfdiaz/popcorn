use super::{Command, Editor};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Bounds, TestAppContext, WindowBounds, WindowOptions, point, px, size};

#[gpui_kit::test]
fn visible_buttons_edit_document_and_native_shortcuts_work(cx: &mut TestAppContext) {
    let (handle, editor) = cx.update(|cx| {
        gpui_kit::init(cx);
        crate::theme::init(cx);
        crate::native_menu::init(cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1440.), px(900.)),
                ))),
                ..crate::window_chrome_options()
            },
            cx,
            |_, cx| cx.new(|_| Editor::default()),
        )
        .unwrap()
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        #[cfg(target_os = "macos")]
        {
            let options = crate::window_chrome_options();
            assert!(options.titlebar.unwrap().appears_transparent);
            assert!(options.app_owns_titlebar_drag);
            let header = window.find("document-header-content").bounds();
            assert!(
                header.left() >= px(84.),
                "header must clear the native traffic lights"
            );
            assert!(
                header.bottom() <= px(crate::TITLEBAR_HEIGHT),
                "header must fit the integrated titlebar"
            );
            let save = window.find("doc-save").bounds();
            assert!(save.top() >= header.top() && save.bottom() <= header.bottom());
        }
        assert!(window.find("drawing-viewport").bounds().size.width > px(500.));
        window.click("add-layer", cx);
        assert_eq!(editor.read(cx).layers.len(), 2);
        window.click("doc-undo", cx);
        assert_eq!(editor.read(cx).layers.len(), 1);
        window.click("doc-redo", cx);
        assert_eq!(editor.read(cx).layers.len(), 2);
        window.scroll(
            "timeline-vertical",
            gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-96.))),
            cx,
        );
        window.click("frame-1-8", cx);
        assert_eq!(editor.read(cx).selected_layer, 1);
        assert_eq!(editor.read(cx).current_frame, 8);
        window.click("add-blank", cx);
        assert_eq!(editor.read(cx).layers[1].keyframes, vec![1, 8]);
        window.press("f6", cx);
        window.click("extend-frames", cx);
        assert_eq!(editor.read(cx).frame_count, 44);
        window.click("canvas-grid", cx);
        assert!(editor.read(cx).grid);
        window.click("edit-document", cx);
        assert!(editor.read(cx).modal.is_some());
        window.click("modal-cancel", cx);
        assert!(editor.read(cx).modal.is_none());
        window.press("enter", cx);
        assert!(editor.read(cx).is_playing);
        window.press("enter", cx);
        assert!(!editor.read(cx).is_playing);
    })
    .unwrap();
}

#[gpui_kit::test]
fn real_pointer_drag_draws_and_context_menu_keeps_frame_target(cx: &mut TestAppContext) {
    let (handle, editor) = cx.update(|cx| {
        gpui_kit::init(cx);
        crate::theme::init(cx);
        crate::native_menu::init(cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1440.), px(900.)),
                ))),
                ..crate::window_chrome_options()
            },
            cx,
            |_, cx| cx.new(|_| Editor::default()),
        )
        .unwrap()
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("tool-Ellipse", cx);
        let b = window.find("drawing-viewport").bounds();
        window.drag(
            b.center() - point(px(120.), px(60.)),
            b.center() + point(px(20.), px(40.)),
            cx,
        );
        let view = editor.read(cx);
        assert_eq!(view.drawing.objects.len(), 1);
        assert!(matches!(
            view.drawing.objects[0].kind,
            super::workspace::Kind::Ellipse
        ));
        assert_eq!(
            [view.drawing.objects[0].w, view.drawing.objects[0].h],
            [140., 100.]
        );
        window.right_click("frame-0-5", cx);
        assert_eq!(editor.read(cx).current_frame, 5);
        assert_eq!(editor.read(cx).selected_layer, 0);
        window.press("escape", cx);
        editor.update(cx, |view, cx| {
            view.apply_edit(Command::Keyframe);
            cx.notify();
        });
        assert!(editor.read(cx).layers[0].keyframes.contains(&5));
    })
    .unwrap();
}
