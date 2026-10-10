//! Document chrome, tool rail and inspector panels.
use super::*;

impl Editor {
    pub(super) fn render_menu_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let menus: Vec<(&str, Vec<(&str, Command)>)> = vec![
            (
                "File",
                vec![
                    ("New…", Command::New),
                    ("Open…", Command::Open),
                    ("Save", Command::Save),
                    ("Save as…", Command::SaveAs),
                ],
            ),
            (
                "Edit",
                vec![
                    ("Undo", Command::Undo),
                    ("Redo", Command::Redo),
                    ("Cut", Command::Cut),
                    ("Copy", Command::Copy),
                    ("Paste", Command::Paste),
                    ("Duplicate", Command::Duplicate),
                    ("Delete", Command::Delete),
                ],
            ),
            (
                "View",
                vec![
                    ("Grid", Command::Grid),
                    ("Fit stage", Command::Fit),
                    ("Zoom in", Command::ZoomIn),
                    ("Zoom out", Command::ZoomOut),
                ],
            ),
            (
                "Insert",
                vec![
                    ("Layer", Command::AddLayer),
                    ("Keyframe (F6)", Command::Keyframe),
                    ("Blank keyframe (F7)", Command::BlankKeyframe),
                    ("Extend timeline (F5)", Command::Extend),
                ],
            ),
            (
                "Window",
                vec![
                    ("Properties", Command::Properties),
                    ("Library", Command::Library),
                    ("Align", Command::Align),
                ],
            ),
            ("Help", vec![("Help", Command::Help)]),
        ];
        h_flex()
            .h_8()
            .flex_shrink_0()
            .px_3()
            .gap_1()
            .bg(theme::color(theme::SURFACE))
            .children(menus.into_iter().map(|(label, commands)| {
                let weak = cx.weak_entity();
                Button::new(format!("menu-{label}"))
                    .ghost()
                    .compact()
                    .label(label.to_string())
                    .dropdown_menu(move |menu, _, cx| {
                        weak.update(cx, |editor, cx| editor.popup(menu, &commands, cx))
                            .expect("editor alive")
                    })
            }))
            .into_any_element()
    }

    pub(super) fn render_document_bar(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled.pop".into());
        let dirty = serde_json::to_string(&self.snapshot()).unwrap_or_default() != self.saved;
        window.set_window_title(&format!(
            "Popcorn — {name}{}",
            if dirty { " *" } else { "" }
        ));
        let content = h_flex()
            .id("document-header-content")
            .test_support()
            .h_full()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .pr_3()
            .when(!cfg!(target_os = "macos"), |bar| bar.pl_3())
            .gap_2()
            .bg(theme::color(theme::SURFACE_ALT))
            .child(
                Icon::new(IconName::File)
                    .small()
                    .text_color(theme::color(theme::ACCENT)),
            )
            .child(div().text_sm().child(name))
            .child(div().text_xs().child(if dirty { "*" } else { "" }))
            .child(
                Icon::new(IconName::ChevronRight)
                    .small()
                    .text_color(theme::color(theme::TEXT_DIM)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme::color(theme::TEXT_MUTED))
                    .child("Scene 1"),
            )
            .child(div().flex_1())
            .child(
                self.command_button("doc-undo", IconName::Undo2, "Undo", Command::Undo, cx)
                    .disabled(self.history.is_empty()),
            )
            .child(
                self.command_button("doc-redo", IconName::Redo2, "Redo", Command::Redo, cx)
                    .disabled(self.future.is_empty()),
            )
            .child(self.command_button(
                "doc-open",
                IconName::FolderOpen,
                "Open document",
                Command::Open,
                cx,
            ))
            .child(self.command_button(
                "doc-save",
                IconName::File,
                "Save document",
                Command::Save,
                cx,
            ))
            .child(
                Button::new("publish")
                    .secondary()
                    .compact()
                    .label(if self.is_playing { "Pause" } else { "Preview" })
                    .icon(if self.is_playing {
                        IconName::Pause
                    } else {
                        IconName::Play
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_playback(cx))),
            )
            .into_any_element();

        #[cfg(target_os = "macos")]
        {
            // One unified header: the native traffic lights live in this inset.
            // TitleBar supplies native dragging and the user's double-click action.
            gpui_kit::component::TitleBar::new()
                .h(px(crate::TITLEBAR_HEIGHT))
                .pl(px(if window.is_fullscreen() { 0. } else { 84. }))
                .bg(theme::color(theme::SURFACE_ALT))
                .border_color(theme::color(theme::BORDER))
                .child(content)
                .into_any_element()
        }
        #[cfg(not(target_os = "macos"))]
        {
            div()
                .h_10()
                .flex_shrink_0()
                .border_b_1()
                .border_color(theme::color(theme::BORDER))
                .child(content)
                .into_any_element()
        }
    }

    pub(super) fn render_tool_rail(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w_12()
            .flex_shrink_0()
            .items_center()
            .py_2()
            .gap_1()
            .bg(theme::color(theme::SURFACE))
            .border_r_1()
            .border_color(theme::color(theme::BORDER))
            .children(Tool::ALL.into_iter().map(|tool| {
                Button::new(format!("tool-{}", tool.label()))
                    .ghost()
                    .compact()
                    .selected(tool == self.selected_tool)
                    .toggled(tool == self.selected_tool)
                    .icon(Icon::new(tool.icon()).small())
                    .tooltip(tool.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_tool = tool;
                        cx.notify();
                    }))
            }))
            .child(div().flex_1())
            .child(self.command_button(
                "tool-settings",
                IconName::Settings2,
                "Document settings",
                Command::DocumentSettings,
                cx,
            ))
            .child(self.command_button(
                "tool-help",
                IconName::CircleQuestionMark,
                "Help",
                Command::Help,
                cx,
            ))
            .into_any_element()
    }

    pub(super) fn render_canvas_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h_12()
            .px_3()
            .gap_2()
            .items_center()
            .bg(theme::color(theme::SURFACE))
            .border_b_1()
            .border_color(theme::color(theme::BORDER))
            .child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(theme::color(theme::ACCENT_SOFT))
                    .text_xs()
                    .text_color(theme::color(theme::ACCENT))
                    .child(self.selected_tool.label()),
            )
            .child(
                self.command_button("canvas-grid", IconName::Grid2x2, "Grid", Command::Grid, cx)
                    .toggled(self.grid),
            )
            .child(self.command_button(
                "canvas-fit",
                IconName::Maximize,
                "Fit stage",
                Command::Fit,
                cx,
            ))
            .child(div().flex_1())
            .child(self.command_button(
                "zoom-out",
                IconName::Minus,
                "Zoom out",
                Command::ZoomOut,
                cx,
            ))
            .child(
                div()
                    .w_16()
                    .text_center()
                    .text_xs()
                    .child(format!("{}%", self.zoom)),
            )
            .child(self.command_button("zoom-in", IconName::Plus, "Zoom in", Command::ZoomIn, cx))
            .into_any_element()
    }

    pub(super) fn render_inspector(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w(px(286.))
            .flex_shrink_0()
            .bg(theme::color(theme::SURFACE))
            .border_l_1()
            .border_color(theme::color(theme::BORDER))
            .child(
                h_flex()
                    .h_10()
                    .flex_shrink_0()
                    .items_center()
                    .px_2()
                    .gap_1()
                    .border_b_1()
                    .border_color(theme::color(theme::BORDER))
                    .children(InspectorTab::ALL.into_iter().map(|tab| {
                        Button::new(format!("inspector-{}", tab.label()))
                            .ghost()
                            .compact()
                            .selected(tab == self.inspector_tab)
                            .label(tab.label())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.inspector_tab = tab;
                                cx.notify();
                            }))
                    })),
            )
            .child(match self.inspector_tab {
                InspectorTab::Properties => self.render_properties(cx),
                InspectorTab::Library => self.render_library(cx),
                InspectorTab::Align => self.render_align(cx),
            })
            .into_any_element()
    }

    fn render_properties(&self, cx: &mut Context<Self>) -> AnyElement {
        let selection = self.editable_selection().map(|i| &self.drawing.objects[i]);
        v_flex()
            .flex_1()
            .min_h_0()
            .id("properties-scroll")
            .overflow_y_scroll()
            .child(
                self.inspector_section(
                    "Document",
                    v_flex()
                        .gap_2()
                        .child(self.property_row("Frame rate", format!("{} fps", self.fps)))
                        .child(self.property_row(
                            "Stage",
                            format!("{} × {} px", self.stage_size[0], self.stage_size[1]),
                        ))
                        .child(
                            self.command_button(
                                "edit-document",
                                IconName::Settings2,
                                "Edit document settings",
                                Command::DocumentSettings,
                                cx,
                            )
                            .label("Edit…"),
                        ),
                ),
            )
            .child(
                self.inspector_section(
                    "Selection",
                    v_flex()
                        .gap_2()
                        .child(
                            self.property_row(
                                "Layer",
                                self.layers[self.selected_layer].name.clone(),
                            ),
                        )
                        .child(
                            self.property_row(
                                "Position",
                                selection
                                    .map(|o| format!("{:.1}, {:.1}", o.x, o.y))
                                    .unwrap_or_else(|| "—".into()),
                            ),
                        )
                        .child(
                            self.property_row(
                                "Size",
                                selection
                                    .map(|o| format!("{:.1} × {:.1}", o.w, o.h))
                                    .unwrap_or_else(|| "—".into()),
                            ),
                        )
                        .child(
                            self.property_row(
                                "Rotation",
                                selection
                                    .map(|o| format!("{:.1}°", o.rotation.to_degrees()))
                                    .unwrap_or_else(|| "—".into()),
                            ),
                        )
                        .child(
                            self.command_button(
                                "edit-object",
                                IconName::Pencil,
                                "Edit selected object",
                                Command::ObjectSettings,
                                cx,
                            )
                            .label("Edit…")
                            .disabled(selection.is_none()),
                        ),
                ),
            )
            .child(
                self.inspector_section(
                    "Fill color",
                    v_flex()
                        .gap_2()
                        .child(
                            h_flex().gap_2().children(
                                [theme::ACCENT, theme::BLUE, theme::GREEN, 0x222222, 0xffffff]
                                    .into_iter()
                                    .map(|color| {
                                        div()
                                            .id(format!("fill-{color}"))
                                            .size_6()
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(theme::color(theme::TEXT_DIM))
                                            .bg(theme::color(color))
                                            .cursor_pointer()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.checkpoint();
                                                this.fill = color;
                                                if let Some(i) = this.editable_selection() {
                                                    this.drawing.objects[i].fill = color;
                                                }
                                                cx.notify();
                                            }))
                                    }),
                            ),
                        )
                        .child(self.property_row("Current", format!("#{:06X}", self.fill))),
                ),
            )
            .into_any_element()
    }

    fn render_library(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex().flex_1().min_h_0().id("library-scroll").overflow_y_scroll().p_3().gap_2()
            .child(self.command_button("save-symbol",IconName::Plus,"Store selection in library",Command::SaveSymbol,cx).label("Store selection").disabled(self.editable_selection().is_none()))
            .when(self.symbols.is_empty(),|d|d.child(div().text_xs().text_color(theme::color(theme::TEXT_MUTED)).child("Select an object and store it here. Click an item to place a copy on the current layer and keyframe.")))
            .children(self.symbols.iter().enumerate().map(|(i,o)|Button::new(format!("symbol-{i}")).ghost().label(format!("Object {} · {:.0} × {:.0}",i+1,o.w,o.h)).on_click(cx.listener(move|this,_,window,cx|this.command(Command::PlaceSymbol(i),window,cx)))))
            .into_any_element()
    }

    fn render_align(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .p_3()
            .gap_2()
            .child(div().text_xs().child("Align selected object to stage"))
            .children(
                [
                    ("align-left", IconName::ArrowLeft, "Left", Command::Left),
                    ("align-right", IconName::ArrowRight, "Right", Command::Right),
                    ("align-top", IconName::ArrowUp, "Top", Command::Top),
                    (
                        "align-bottom",
                        IconName::ArrowDown,
                        "Bottom",
                        Command::Bottom,
                    ),
                    (
                        "align-center-x",
                        IconName::PanelLeft,
                        "Horizontal center",
                        Command::CenterX,
                    ),
                    (
                        "align-center-y",
                        IconName::PanelRight,
                        "Vertical center",
                        Command::CenterY,
                    ),
                    (
                        "bring-front",
                        IconName::Plus,
                        "Bring to front",
                        Command::Front,
                    ),
                    ("send-back", IconName::Minus, "Send to back", Command::Back),
                ]
                .into_iter()
                .map(|(id, icon, label, command)| {
                    self.command_button(id, icon, label, command, cx)
                        .label(label)
                        .disabled(self.editable_selection().is_none())
                }),
            )
            .into_any_element()
    }

    pub(super) fn render_timeline_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.weak_entity();
        h_flex()
            .h_10()
            .flex_shrink_0()
            .items_center()
            .px_3()
            .gap_2()
            .bg(theme::color(theme::SURFACE_ALT))
            .border_b_1()
            .border_color(theme::color(theme::BORDER))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Timeline"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme::color(theme::TEXT_DIM))
                    .child(format!("{} frames", self.frame_count)),
            )
            .child(div().flex_1())
            .child(self.command_button(
                "add-layer",
                IconName::Plus,
                "New layer",
                Command::AddLayer,
                cx,
            ))
            .child(self.command_button(
                "add-keyframe",
                IconName::Circle,
                "Insert keyframe (F6)",
                Command::Keyframe,
                cx,
            ))
            .child(self.command_button(
                "add-blank",
                IconName::Square,
                "Insert blank keyframe (F7)",
                Command::BlankKeyframe,
                cx,
            ))
            .child(self.command_button(
                "extend-frames",
                IconName::ArrowRight,
                "Extend timeline (F5)",
                Command::Extend,
                cx,
            ))
            .child(
                Button::new("timeline-options")
                    .ghost()
                    .compact()
                    .icon(IconName::Ellipsis)
                    .tooltip("Timeline options")
                    .dropdown_menu(move |menu, _, cx| {
                        weak.update(cx, |editor, cx| {
                            editor.popup(
                                menu,
                                &[
                                    ("Rename layer…", Command::RenameLayer),
                                    ("Move layer up", Command::LayerUp),
                                    ("Move layer down", Command::LayerDown),
                                    ("Delete layer", Command::DeleteLayer),
                                    ("Clear keyframe", Command::ClearKeyframe),
                                ],
                                cx,
                            )
                        })
                        .expect("editor alive")
                    }),
            )
            .into_any_element()
    }
}
