use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::base::{Disableable, TestSupportExt};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Icon, Selectable as _, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::theme;
mod chrome;
mod commands;
mod geometry;
mod panels;
#[cfg(test)]
mod tests;
mod workspace;
use commands::Command;
use gpui_kit::component::color_picker::{ColorPickerEvent, ColorPickerState};
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu};

const FRAME_COUNT: usize = 32;
const FRAME_WIDTH: f32 = 28.;
const LAYER_WIDTH: f32 = 184.;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    Pencil,
    Eraser,
    Fill,
    Rectangle,
    Ellipse,
    Text,
    Hand,
    Zoom,
}

impl Tool {
    const ALL: [Self; 9] = [
        Self::Select,
        Self::Pencil,
        Self::Eraser,
        Self::Fill,
        Self::Rectangle,
        Self::Ellipse,
        Self::Text,
        Self::Hand,
        Self::Zoom,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Select => "Selection",
            Self::Pencil => "Pencil",
            Self::Eraser => "Eraser",
            Self::Fill => "Fill",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Text => "Text",
            Self::Hand => "Hand",
            Self::Zoom => "Zoom",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Select => IconName::Move,
            Self::Pencil => IconName::Pencil,
            Self::Eraser => IconName::Eraser,
            Self::Fill => IconName::PaintBucket,
            Self::Rectangle => IconName::Square,
            Self::Ellipse => IconName::Circle,
            Self::Text => IconName::ALargeSmall,
            Self::Hand => IconName::Hand,
            Self::Zoom => IconName::Search,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InspectorTab {
    Properties,
    Library,
    Align,
}

#[derive(Clone, Copy)]
struct TimelineDrag {
    last_frame: usize,
}

impl InspectorTab {
    const ALL: [Self; 3] = [Self::Properties, Self::Library, Self::Align];

    fn label(self) -> &'static str {
        match self {
            Self::Properties => "Properties",
            Self::Library => "Library",
            Self::Align => "Align",
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct LayerState {
    name: String,
    color: u32,
    visible: bool,
    locked: bool,
    keyframes: Vec<usize>,
}

impl LayerState {
    fn new(name: &str, color: u32) -> Self {
        Self {
            name: name.into(),
            color,
            visible: true,
            locked: false,
            keyframes: vec![1],
        }
    }
}

pub struct Editor {
    history: Vec<commands::Snapshot>,
    future: Vec<commands::Snapshot>,
    clipboard: Option<Vec<workspace::Object>>,
    symbols: Vec<workspace::Object>,
    frame_count: usize,
    fps: u32,
    stage_size: [f32; 2],
    fill: u32,
    grid: bool,
    onion: bool,
    looping: bool,
    status: String,
    path: Option<std::path::PathBuf>,
    saved: String,
    modal: Option<panels::Modal>,
    drawing: workspace::Drawing,
    focus: Option<FocusHandle>,
    current_frame: usize,
    is_playing: bool,
    selected_tool: Tool,
    eraser_size: f32,
    inspector_tab: InspectorTab,
    selected_layer: usize,
    zoom: u32,
    layers: Vec<LayerState>,
    selected_frames: Vec<(usize, usize)>,
    selected_layers: Vec<usize>,
    frame_selection_anchor: Option<(usize, usize)>,
    layer_selection_anchor: Option<usize>,
    timeline_drag: Option<TimelineDrag>,
    timeline_pointer_handled: bool,
    text_editing: Option<usize>,
    playback_task: Option<Task<()>>,
    color_picker: Option<Entity<ColorPickerState>>,
    color_picker_subscription: Option<Subscription>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            future: Vec::new(),
            clipboard: None,
            symbols: Vec::new(),
            frame_count: FRAME_COUNT,
            fps: 24,
            stage_size: [800., 450.],
            fill: theme::ACCENT,
            grid: false,
            onion: false,
            looping: true,
            status: "Ready".into(),
            path: None,
            saved: String::new(),
            modal: None,
            drawing: workspace::Drawing::default(),
            focus: None,
            current_frame: 1,
            is_playing: false,
            selected_tool: Tool::Select,
            eraser_size: 20.,
            inspector_tab: InspectorTab::Properties,
            selected_layer: 0,
            zoom: 100,
            layers: vec![LayerState::new("Layer 1", theme::ACCENT)],
            selected_frames: Vec::new(),
            selected_layers: vec![0],
            frame_selection_anchor: None,
            layer_selection_anchor: Some(0),
            timeline_drag: None,
            timeline_pointer_handled: false,
            text_editing: None,
            playback_task: None,
            color_picker: None,
            color_picker_subscription: None,
        }
    }
}

// Keep the native menus and on-screen controls on the same command path.
macro_rules! bind_commands {
    ($view:expr,$cx:expr; $($action:ident => $command:ident),* $(,)?) => {
        $view$(.on_action($cx.listener(|this, _: &crate::native_menu::$action, window, cx|this.command(Command::$command,window,cx))))*
    };
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_color_picker(window, cx);
        let focus = self
            .focus
            .get_or_insert_with(|| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                focus
            })
            .clone();
        let root = div()
            .relative()
            .track_focus(&focus)
            .key_context("Editor")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.cancel_interaction();
                    cx.stop_propagation();
                    cx.notify();
                } else if this.edit_text(event) {
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_action(
                cx.listener(|this, _: &crate::native_menu::TogglePlayback, _, cx| {
                    this.toggle_playback(cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::PreviousFrame, _, cx| {
                    this.select_frame(
                        this.selected_layer,
                        this.current_frame.saturating_sub(1).max(1),
                    );
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::NextFrame, _, cx| {
                    this.select_frame(
                        this.selected_layer,
                        (this.current_frame + 1).min(this.frame_count),
                    );
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::JumpFirstFrame, _, cx| {
                    this.select_frame(this.selected_layer, 1);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::JumpLastFrame, _, cx| {
                    this.select_frame(this.selected_layer, this.frame_count);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::SelectTool, _, cx| {
                    this.select_tool(Tool::Select, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::PencilTool, _, cx| {
                    this.select_tool(Tool::Pencil, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::EraserTool, _, cx| {
                    this.select_tool(Tool::Eraser, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::FillTool, _, cx| {
                    this.select_tool(Tool::Fill, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::RectangleTool, _, cx| {
                    this.select_tool(Tool::Rectangle, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::EllipseTool, _, cx| {
                    this.select_tool(Tool::Ellipse, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::TextTool, _, cx| {
                    this.select_tool(Tool::Text, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::HandTool, _, cx| {
                    this.select_tool(Tool::Hand, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::native_menu::ZoomTool, _, cx| {
                    this.select_tool(Tool::Zoom, cx);
                }),
            )
            .on_action(|_: &crate::native_menu::CloseWindow, window, _| window.remove_window())
            .on_action(|_: &crate::native_menu::MinimizeWindow, window, _| window.minimize_window())
            .on_action(|_: &crate::native_menu::ZoomWindow, window, _| window.zoom_window());
        bind_commands!(root,cx;
            NewDocument=>New,OpenDocument=>Open,SaveDocument=>Save,SaveAsDocument=>SaveAs,
            Undo=>Undo,Redo=>Redo,Copy=>Copy,Cut=>Cut,Paste=>Paste,Duplicate=>Duplicate,DeleteSelection=>Delete,
            AddLayer=>AddLayer,DeleteLayer=>DeleteLayer,InsertKeyframe=>Keyframe,BlankKeyframe=>BlankKeyframe,ClearKeyframe=>ClearKeyframe,DeleteSelectedFrames=>DeleteFrames,MoveSelectedFramesLeft=>MoveFramesLeft,MoveSelectedFramesRight=>MoveFramesRight,ExtendTimeline=>Extend,
            ToggleGrid=>Grid,FitStage=>Fit,ZoomIn=>ZoomIn,ZoomOut=>ZoomOut,ToggleOnion=>Onion,ToggleLoop=>Loop,
            ShowProperties=>Properties,ShowLibrary=>Library,ShowAlign=>Align,Help=>Help,About=>About
        )
        .flex().flex_col().size_full().overflow_hidden().bg(theme::color(theme::APP_BG)).text_color(theme::color(theme::TEXT))
        .when(!cfg!(target_os="macos"),|d|d.child(self.render_menu_bar(cx)))
        .child(self.render_document_bar(window,cx))
        .child(h_flex().items_stretch().flex_1().min_h_0().overflow_hidden().child(self.render_tool_rail(cx)).child(self.render_workspace(cx)).child(self.render_inspector(cx)))
        .child(self.render_timeline(cx)).child(self.render_status_bar())
        .when(self.modal.is_some(),|d|d.child(self.render_modal(cx)))
    }
}

impl Editor {
    fn select_tool(&mut self, tool: Tool, cx: &mut Context<Self>) {
        self.cancel_interaction();
        if matches!(tool, Tool::Eraser | Tool::Fill) {
            self.clear_selection();
        }
        self.selected_tool = tool;
        cx.notify();
    }

    fn ensure_color_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.color_picker.is_some() {
            return;
        }
        let state =
            cx.new(|cx| ColorPickerState::new(window, cx).default_value(theme::color(self.fill)));
        let subscription = cx.subscribe_in(
            &state,
            window,
            move |editor, _, event: &ColorPickerEvent, _, cx| {
                let ColorPickerEvent::Change(Some(color)) = event else {
                    return;
                };
                let rgba = Rgba::from(*color);
                let packed = ((rgba.r.clamp(0., 1.) * 255.).round() as u32) << 16
                    | ((rgba.g.clamp(0., 1.) * 255.).round() as u32) << 8
                    | (rgba.b.clamp(0., 1.) * 255.).round() as u32;
                editor.checkpoint();
                editor.fill = packed;
                let selected = editor.editable_selections();
                for i in selected {
                    editor.drawing.objects[i].fill = packed;
                }
                cx.notify();
            },
        );
        self.color_picker = Some(state);
        self.color_picker_subscription = Some(subscription);
    }

    fn set_fill_color(&mut self, color: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.checkpoint();
        self.fill = color;
        let selected = self.editable_selections();
        for i in selected {
            self.drawing.objects[i].fill = color;
        }
        if let Some(state) = &self.color_picker {
            state.update(cx, |state, cx| {
                state.set_value(theme::color(color), window, cx)
            });
        }
    }

    fn render_timeline(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .h(px(238.))
            .flex_shrink_0()
            .bg(theme::color(theme::SURFACE))
            .border_t_1()
            .border_color(theme::color(theme::BORDER))
            .child(self.render_timeline_header(cx))
            .child(self.render_timeline_body(cx))
            .into_any_element()
    }

    fn render_timeline_body(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .id("timeline-vertical")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .test_support()
                    .child(
                        h_flex()
                            .items_start()
                            .child(
                                v_flex()
                                    .w(px(LAYER_WIDTH))
                                    .flex_shrink_0()
                                    .child(div().h_7().child(self.timeline_corner()))
                                    .child(self.render_layer_list(cx)),
                            )
                            .child(
                                div()
                                    .id("timeline-horizontal")
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_x_scroll()
                                    .child(
                                        v_flex()
                                            .w(px(FRAME_WIDTH * self.frame_count as f32))
                                            .child(div().h_7().child(self.render_frame_ruler(cx)))
                                            .child(self.render_frame_grid(cx)),
                                    ),
                            ),
                    ),
            )
            .child(self.render_transport(cx))
            .into_any_element()
    }

    fn timeline_corner(&self) -> AnyElement {
        h_flex()
            .w(px(LAYER_WIDTH))
            .h_full()
            .flex_shrink_0()
            .items_center()
            .px_3()
            .gap_2()
            .bg(theme::color(theme::SURFACE))
            .border_r_1()
            .border_color(theme::color(theme::BORDER))
            .child(
                Icon::new(IconName::Layers)
                    .small()
                    .text_color(theme::color(theme::TEXT_MUTED)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme::color(theme::TEXT_MUTED))
                    .child("Layers"),
            )
            .into_any_element()
    }

    fn render_frame_ruler(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .bg(theme::color(theme::SURFACE))
            .children((1..=self.frame_count).map(|frame| {
                let is_current = frame == self.current_frame;
                let is_selected = self
                    .selected_frames
                    .iter()
                    .any(|(_, selected_frame)| *selected_frame == frame);
                div()
                    .w(px(FRAME_WIDTH))
                    .h_full()
                    .flex_shrink_0()
                    .flex()
                    .items_end()
                    .justify_center()
                    .pb_1()
                    .border_r_1()
                    .border_color(theme::color(theme::GRID))
                    .when(is_current || is_selected, |this| {
                        this.bg(theme::color(if is_current {
                            theme::ACCENT_SOFT
                        } else {
                            0x3a2d3a
                        }))
                    })
                    .text_xs()
                    .text_color(if is_current {
                        theme::color(theme::ACCENT)
                    } else {
                        theme::color(theme::TEXT_DIM)
                    })
                    .child(if frame % 5 == 0 || frame == 1 {
                        frame.to_string()
                    } else {
                        String::new()
                    })
                    .id(format!("ruler-{frame}"))
                    .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                        this.select_timeline_frame(
                            this.selected_layer,
                            frame,
                            event.modifiers().shift,
                        );
                        cx.notify();
                    }))
            }))
            .into_any_element()
    }

    fn render_layer_list(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w(px(LAYER_WIDTH))
            .flex_shrink_0()
            .bg(theme::color(theme::SURFACE))
            .border_r_1()
            .border_color(theme::color(theme::BORDER))
            .children(self.layers.iter().enumerate().map(|(index, layer)| {
                let selected = self.is_layer_selected(index);
                h_flex()
                    .h_8()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .bg(if selected {
                        theme::color(theme::ACCENT_SOFT)
                    } else {
                        theme::color(theme::SURFACE)
                    })
                    .border_b_1()
                    .border_color(theme::color(theme::GRID))
                    .child(
                        div()
                            .w_1p5()
                            .h_5()
                            .rounded_full()
                            .bg(theme::color(layer.color)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(if selected {
                                theme::color(theme::TEXT)
                            } else {
                                theme::color(theme::TEXT_MUTED)
                            })
                            .child(layer.name.clone()),
                    )
                    .child(self.layer_toggle(index, IconName::Eye, layer.visible, cx))
                    .child(self.layer_toggle(index, IconName::Lock, layer.locked, cx))
                    .id(format!("layer-{index}"))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            this.context_select_timeline_layer(index, event.modifiers.shift);
                            cx.notify();
                        }),
                    )
                    .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                        this.select_timeline_layer(index, event.modifiers().shift);
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
                                        ("Rename…", Command::RenameLayer),
                                        ("New layer", Command::AddLayer),
                                        ("Move up", Command::LayerUp),
                                        ("Move down", Command::LayerDown),
                                        ("Delete layer", Command::DeleteLayer),
                                    ],
                                    cx,
                                )
                            })
                            .expect("editor alive")
                        }
                    })
            }))
            .into_any_element()
    }

    fn render_frame_grid(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .children(self.layers.iter().enumerate().map(|(layer_index, layer)| {
                h_flex()
                    .h_8()
                    .flex_shrink_0()
                    .children((1..=self.frame_count).map(|frame| {
                        let current = frame == self.current_frame;
                        let selected = self.is_frame_selected(layer_index, frame);
                        let active = layer
                            .keyframes
                            .iter()
                            .copied()
                            .take_while(|f| *f <= frame)
                            .last()
                            .unwrap_or(1);
                        let filled = self
                            .drawing
                            .objects
                            .iter()
                            .any(|o| o.layer == layer_index && o.keyframe == active);
                        let keyframe = layer.keyframes.contains(&frame) && filled;
                        div()
                            .relative()
                            .w(px(FRAME_WIDTH))
                            .h_full()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_r_1()
                            .border_b_1()
                            .border_color(theme::color(theme::GRID))
                            .bg(if selected {
                                theme::color(0x4a2d39)
                            } else if current {
                                theme::color(0x4a302d)
                            } else {
                                theme::color(theme::SURFACE)
                            })
                            .when(filled, |this| {
                                this.child(
                                    div()
                                        .w_full()
                                        .h_3()
                                        .bg(theme::color(layer.color).alpha(0.34)),
                                )
                            })
                            .when(keyframe, |this| {
                                this.child(
                                    div()
                                        .absolute()
                                        .w_2()
                                        .h_2()
                                        .rounded_full()
                                        .border_1()
                                        .border_color(theme::color(layer.color))
                                        .when(filled, |d| d.bg(theme::color(layer.color))),
                                )
                            })
                            .when(current, |this| {
                                this.border_l_1().border_color(theme::color(theme::ACCENT))
                            })
                            .when(selected, |this| {
                                this.border_1().border_color(theme::color(theme::ACCENT))
                            })
                            .id(format!("frame-{layer_index}-{frame}"))
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                    this.context_select_timeline_frame(
                                        layer_index,
                                        frame,
                                        event.modifiers.shift,
                                    );
                                    cx.notify();
                                }),
                            )
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                    this.begin_timeline_drag(
                                        layer_index,
                                        frame,
                                        event.modifiers.shift,
                                    );
                                    cx.notify();
                                }),
                            )
                            .on_mouse_move(cx.listener(
                                move |this, event: &MouseMoveEvent, _, cx| {
                                    if event.dragging() {
                                        this.update_timeline_drag(layer_index, frame);
                                        cx.notify();
                                    }
                                },
                            ))
                            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                if this.timeline_pointer_handled {
                                    this.timeline_pointer_handled = false;
                                } else {
                                    this.select_timeline_frame(
                                        layer_index,
                                        frame,
                                        event.modifiers().shift,
                                    );
                                }
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
                                                ("Insert keyframe (F6)", Command::Keyframe),
                                                (
                                                    "Insert blank keyframe (F7)",
                                                    Command::BlankKeyframe,
                                                ),
                                                ("Clear keyframe", Command::ClearKeyframe),
                                                ("Delete selected frames", Command::DeleteFrames),
                                                (
                                                    "Move selected frames left",
                                                    Command::MoveFramesLeft,
                                                ),
                                                (
                                                    "Move selected frames right",
                                                    Command::MoveFramesRight,
                                                ),
                                                ("Extend timeline", Command::Extend),
                                            ],
                                            cx,
                                        )
                                    })
                                    .expect("editor alive")
                                }
                            })
                    }))
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.finish_timeline_drag();
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    fn render_transport(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .h_9()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .px_3()
            .bg(theme::color(theme::SURFACE_ALT))
            .border_t_1()
            .border_color(theme::color(theme::BORDER))
            .child(self.action_button(
                IconName::ArrowLeft,
                "Previous frame",
                "prev-frame",
                false,
                cx.listener(|this, _, _, cx| {
                    this.current_frame = this.current_frame.saturating_sub(1).max(1);
                    cx.notify();
                }),
            ))
            .child(self.play_button(cx))
            .child(self.action_button(
                IconName::ArrowRight,
                "Next frame",
                "next-frame",
                false,
                cx.listener(|this, _, _, cx| {
                    this.current_frame = (this.current_frame + 1).min(this.frame_count);
                    cx.notify();
                }),
            ))
            .child(div().w_px().h_5().mx_2().bg(theme::color(theme::BORDER)))
            .child(
                div()
                    .text_xs()
                    .text_color(theme::color(theme::TEXT_MUTED))
                    .child("Frame"),
            )
            .child(
                div()
                    .w_12()
                    .h_6()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .bg(theme::color(theme::APP_BG))
                    .text_xs()
                    .text_color(theme::color(theme::ACCENT))
                    .child(format!("{} / {}", self.current_frame, self.frame_count)),
            )
            .child(div().flex_1())
            .child(
                self.command_button(
                    "onion-skin",
                    IconName::Layers,
                    "Onion skin",
                    Command::Onion,
                    cx,
                )
                .toggled(self.onion),
            )
            .child(
                self.command_button(
                    "loop-playback",
                    IconName::Redo2,
                    "Loop playback",
                    Command::Loop,
                    cx,
                )
                .toggled(self.looping),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme::color(theme::TEXT_DIM))
                    .child(format!("{} fps", self.fps)),
            )
            .child(div().w_2())
            .into_any_element()
    }

    fn render_status_bar(&self) -> AnyElement {
        h_flex()
            .h_6()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .px_3()
            .bg(theme::color(0x121318))
            .border_t_1()
            .border_color(theme::color(theme::BORDER))
            .text_xs()
            .text_color(theme::color(theme::TEXT_DIM))
            .child(
                h_flex()
                    .gap_3()
                    .child(self.status.clone())
                    .child("Scene 1")
                    .child(format!("{}% zoom", self.zoom)),
            )
            .child(format!(
                "{} objects · {} layers",
                self.drawing.objects.len(),
                self.layers.len()
            ))
            .into_any_element()
    }

    fn play_button(&self, cx: &mut Context<Self>) -> Button {
        let icon = if self.is_playing {
            IconName::Pause
        } else {
            IconName::Play
        };
        Button::new("playback")
            .primary()
            .compact()
            .icon(icon)
            .toggled(self.is_playing)
            .tooltip(if self.is_playing { "Pause" } else { "Play" })
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_playback(cx);
            }))
    }

    fn action_button(
        &self,
        icon: IconName,
        tooltip: &'static str,
        id: &'static str,
        selected: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Button {
        Button::new(id)
            .ghost()
            .compact()
            .selected(selected)
            .icon(icon)
            .tooltip(tooltip)
            .on_click(on_click)
    }

    fn layer_toggle(
        &self,
        index: usize,
        icon: IconName,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(format!("layer-{index}-{icon:?}"))
            .ghost()
            .compact()
            .icon(
                Icon::new(if icon == IconName::Eye && !enabled {
                    IconName::EyeOff
                } else if icon == IconName::Lock && !enabled {
                    IconName::LockOpen
                } else {
                    icon
                })
                .xsmall(),
            )
            .toggled(enabled)
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.checkpoint();
                if icon == IconName::Eye {
                    this.layers[index].visible = !this.layers[index].visible;
                } else {
                    this.layers[index].locked = !this.layers[index].locked;
                }
                this.clear_selection();
                cx.notify();
            }))
    }

    fn inspector_section(&self, title: &'static str, content: impl IntoElement) -> AnyElement {
        v_flex()
            .gap_3()
            .p_3()
            .border_b_1()
            .border_color(theme::color(theme::BORDER))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme::color(theme::TEXT_MUTED))
                    .child(title),
            )
            .child(content)
            .into_any_element()
    }

    fn property_row(&self, label: &'static str, value: impl Into<SharedString>) -> AnyElement {
        h_flex()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(theme::color(theme::TEXT_DIM))
                    .child(label),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme::color(theme::TEXT))
                    .child(value.into()),
            )
            .into_any_element()
    }

    fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        self.is_playing = !self.is_playing;

        if self.is_playing {
            let task = cx.spawn(async move |this, cx| {
                loop {
                    let fps = this.update(cx, |editor, _| editor.fps).unwrap_or(24);
                    cx.background_executor()
                        .timer(Duration::from_secs_f64(1. / fps as f64))
                        .await;
                    let keep_playing = this
                        .update(cx, |editor, cx| {
                            if !editor.is_playing {
                                return false;
                            }
                            if editor.current_frame >= editor.frame_count && !editor.looping {
                                editor.is_playing = false;
                                cx.notify();
                                return false;
                            }
                            editor.drawing.selected = None;
                            editor.current_frame = if editor.current_frame >= editor.frame_count {
                                1
                            } else {
                                editor.current_frame + 1
                            };
                            cx.notify();
                            true
                        })
                        .unwrap_or(false);

                    if !keep_playing {
                        break;
                    }
                }
            });
            self.playback_task = Some(task);
        } else {
            self.playback_task = None;
        }

        cx.notify();
    }
}
