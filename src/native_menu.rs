use gpui_kit::*;

actions!(
    popcorn,
    [
        Quit,
        CloseWindow,
        MinimizeWindow,
        ZoomWindow,
        NewDocument,
        OpenDocument,
        SaveDocument,
        SaveAsDocument,
        Undo,
        Redo,
        Copy,
        Cut,
        Paste,
        Duplicate,
        DeleteSelection,
        AddLayer,
        DeleteLayer,
        InsertKeyframe,
        BlankKeyframe,
        ClearKeyframe,
        ExtendTimeline,
        ToggleGrid,
        FitStage,
        ZoomIn,
        ZoomOut,
        ToggleOnion,
        ToggleLoop,
        ShowProperties,
        ShowLibrary,
        ShowAlign,
        Help,
        TogglePlayback,
        PreviousFrame,
        NextFrame
    ]
);

pub fn init(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    let primary = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    let shortcut = |key: &str| format!("{primary}-{key}");
    cx.bind_keys([
        KeyBinding::new(&shortcut("q"), Quit, None),
        KeyBinding::new(&shortcut("w"), CloseWindow, None),
        KeyBinding::new(&shortcut("m"), MinimizeWindow, None),
        KeyBinding::new(&shortcut("n"), NewDocument, None),
        KeyBinding::new(&shortcut("o"), OpenDocument, None),
        KeyBinding::new(&shortcut("s"), SaveDocument, Some("Editor")),
        KeyBinding::new(&shortcut("shift-s"), SaveAsDocument, Some("Editor")),
        KeyBinding::new(&shortcut("z"), Undo, Some("Editor")),
        KeyBinding::new(&shortcut("shift-z"), Redo, Some("Editor")),
        KeyBinding::new(&shortcut("c"), Copy, Some("Editor")),
        KeyBinding::new(&shortcut("x"), Cut, Some("Editor")),
        KeyBinding::new(&shortcut("v"), Paste, Some("Editor")),
        KeyBinding::new(&shortcut("d"), Duplicate, Some("Editor")),
        KeyBinding::new("backspace", DeleteSelection, Some("Editor")),
        KeyBinding::new("delete", DeleteSelection, Some("Editor")),
        KeyBinding::new(&shortcut("shift-n"), AddLayer, Some("Editor")),
        KeyBinding::new(&shortcut("shift-backspace"), DeleteLayer, Some("Editor")),
        KeyBinding::new("f6", InsertKeyframe, Some("Editor")),
        KeyBinding::new("f7", BlankKeyframe, Some("Editor")),
        KeyBinding::new("shift-f6", ClearKeyframe, Some("Editor")),
        KeyBinding::new("f5", ExtendTimeline, Some("Editor")),
        KeyBinding::new(&shortcut("g"), ToggleGrid, Some("Editor")),
        KeyBinding::new(&shortcut("0"), FitStage, Some("Editor")),
        KeyBinding::new(&shortcut("="), ZoomIn, Some("Editor")),
        KeyBinding::new(&shortcut("shift-="), ZoomIn, Some("Editor")),
        KeyBinding::new(&shortcut("-"), ZoomOut, Some("Editor")),
        KeyBinding::new("shift-o", ToggleOnion, Some("Editor")),
        KeyBinding::new("shift-l", ToggleLoop, Some("Editor")),
        KeyBinding::new(&shortcut("1"), ShowProperties, Some("Editor")),
        KeyBinding::new(&shortcut("2"), ShowLibrary, Some("Editor")),
        KeyBinding::new(&shortcut("3"), ShowAlign, Some("Editor")),
        KeyBinding::new("f1", Help, None),
        KeyBinding::new("space", TogglePlayback, Some("Editor")),
        KeyBinding::new("left", PreviousFrame, Some("Editor")),
        KeyBinding::new("right", NextFrame, Some("Editor")),
    ]);
    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([KeyBinding::new("ctrl-y", Redo, Some("Editor"))]);
    #[cfg(target_os = "macos")]
    cx.set_menus([
        Menu::new("Popcorn").items([MenuItem::action("Quit Popcorn", Quit)]),
        Menu::new("File").items([
            MenuItem::action("New", NewDocument),
            MenuItem::action("Open…", OpenDocument),
            MenuItem::separator(),
            MenuItem::action("Save", SaveDocument),
            MenuItem::action("Save As…", SaveAsDocument),
            MenuItem::separator(),
            MenuItem::action("Close Window", CloseWindow),
        ]),
        Menu::new("Edit").items([
            MenuItem::action("Undo", Undo),
            MenuItem::action("Redo", Redo),
            MenuItem::separator(),
            MenuItem::action("Cut", Cut),
            MenuItem::action("Copy", Copy),
            MenuItem::action("Paste", Paste),
            MenuItem::separator(),
            MenuItem::action("Duplicate", Duplicate),
            MenuItem::action("Delete Selection", DeleteSelection),
        ]),
        Menu::new("View").items([
            MenuItem::action("Show / Hide Grid", ToggleGrid),
            MenuItem::action("Show / Hide Onion Skin", ToggleOnion),
            MenuItem::separator(),
            MenuItem::action("Fit Stage", FitStage),
            MenuItem::action("Zoom In", ZoomIn),
            MenuItem::action("Zoom Out", ZoomOut),
        ]),
        Menu::new("Insert").items([
            MenuItem::action("Layer", AddLayer),
            MenuItem::separator(),
            MenuItem::action("Keyframe", InsertKeyframe),
            MenuItem::action("Blank Keyframe", BlankKeyframe),
            MenuItem::action("Extend Timeline", ExtendTimeline),
        ]),
        Menu::new("Modify").items([
            MenuItem::action("Clear Keyframe", ClearKeyframe),
            MenuItem::action("Delete Layer", DeleteLayer),
        ]),
        Menu::new("Control").items([
            MenuItem::action("Play / Pause", TogglePlayback),
            MenuItem::action("Previous Frame", PreviousFrame),
            MenuItem::action("Next Frame", NextFrame),
            MenuItem::separator(),
            MenuItem::action("Loop Playback", ToggleLoop),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", MinimizeWindow),
            MenuItem::action("Zoom", ZoomWindow),
            MenuItem::separator(),
            MenuItem::action("Properties", ShowProperties),
            MenuItem::action("Library", ShowLibrary),
            MenuItem::action("Align", ShowAlign),
        ]),
        Menu::new("Help").items([MenuItem::action("Popcorn Help", Help)]),
    ]);
}
