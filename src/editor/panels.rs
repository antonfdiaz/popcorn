use super::*;
use gpui_kit::component::input::{Input, InputState};

enum Form {
    Document,
    Object,
    Rename,
    New,
    Help,
}
pub(super) struct Modal {
    form: Form,
    title: &'static str,
    fields: Vec<(&'static str, Entity<InputState>)>,
    error: String,
}
impl Modal {
    fn new(
        form: Form,
        title: &'static str,
        values: Vec<(&'static str, String)>,
        window: &mut Window,
        cx: &mut Context<Editor>,
    ) -> Self {
        Self {
            form,
            title,
            error: String::new(),
            fields: values
                .into_iter()
                .map(|(label, value)| {
                    (
                        label,
                        cx.new(|cx| InputState::new(window, cx).default_value(value)),
                    )
                })
                .collect(),
        }
    }
    pub(super) fn document(editor: &Editor, window: &mut Window, cx: &mut Context<Editor>) -> Self {
        Self::new(
            Form::Document,
            "Document settings",
            vec![
                ("Width", editor.stage_size[0].to_string()),
                ("Height", editor.stage_size[1].to_string()),
                ("FPS", editor.fps.to_string()),
            ],
            window,
            cx,
        )
    }
    pub(super) fn object(
        o: &workspace::Object,
        window: &mut Window,
        cx: &mut Context<Editor>,
    ) -> Self {
        let mut fields = vec![
            ("X", o.x.to_string()),
            ("Y", o.y.to_string()),
            ("Width", o.w.to_string()),
            ("Height", o.h.to_string()),
            ("Rotation (degrees)", o.rotation.to_degrees().to_string()),
            ("Fill (#RRGGBB)", format!("{:06x}", o.fill)),
        ];
        if let workspace::Kind::Text(text) = &o.kind {
            fields.push(("Text", text.clone()));
        }
        Self::new(Form::Object, "Object properties", fields, window, cx)
    }
    pub(super) fn rename(name: &str, window: &mut Window, cx: &mut Context<Editor>) -> Self {
        Self::new(
            Form::Rename,
            "Rename layer",
            vec![("Name", name.into())],
            window,
            cx,
        )
    }
    pub(super) fn confirm_new(window: &mut Window, cx: &mut Context<Editor>) -> Self {
        Self::new(Form::New, "New document", vec![], window, cx)
    }
    pub(super) fn help() -> Self {
        Self {
            form: Form::Help,
            title: "Popcorn Help",
            fields: vec![],
            error: String::new(),
        }
    }
}
impl Editor {
    pub(super) fn render_modal(&self, cx: &mut Context<Self>) -> AnyElement {
        let modal = self.modal.as_ref().unwrap();
        div().absolute().inset_0().flex().items_center().justify_center().bg(rgba(0x00000099))
            .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
            .child(v_flex().w(px(400.)).p_5().gap_3().rounded_lg().shadow_lg().bg(theme::color(theme::SURFACE_ALT)).border_1().border_color(theme::color(theme::BORDER))
                .child(div().font_weight(FontWeight::BOLD).child(modal.title))
                .when(matches!(modal.form,Form::New),|d|d.child(div().text_sm().child("Start with an empty stage? Save your current document first if you want to keep it.")))
                .when(matches!(modal.form,Form::Help),|d|d.child(div().text_sm().child("Drag to draw or move. Shift-click selects multiple objects, frames, or layers; drag selected keyframes horizontally to move them together. Group handles scale and rotate objects together. Double-click text to edit it; Escape cancels the current operation. Shift constrains shapes, keeps transform proportions and snaps rotation. Hand pans; Zoom clicks in (Alt: out). Right-click objects, layers or frames for editing commands. Delete removes selected frames or objects. F6: keyframe · F7: blank keyframe · F5: extend · Enter: play. Save/Open use the .pop document format. Library stores reusable copies, not linked symbols.")))
                .children(modal.fields.iter().map(|(label,state)|v_flex().gap_1().child(div().text_xs().text_color(theme::color(theme::TEXT_MUTED)).child(*label)).child(Input::new(state).small())))
                .child(div().text_xs().text_color(theme::color(theme::ACCENT)).child(modal.error.clone()))
                .child(h_flex().gap_2().justify_end()
                    .child(Button::new("modal-cancel").ghost().label("Close").on_click(cx.listener(|this,_,window,cx|{this.modal=None;if let Some(f)=&this.focus {window.focus(f,cx);}cx.notify();})))
                    .when(!matches!(modal.form,Form::Help),|d|d.child(Button::new("modal-apply").primary().label("Apply").on_click(cx.listener(|this,_,window,cx|this.apply_modal(window,cx)))))))
            .into_any_element()
    }
    fn apply_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(modal) = self.modal.take() else {
            return;
        };
        let values: Vec<String> = modal
            .fields
            .iter()
            .map(|(_, s)| s.read(cx).value().to_string())
            .collect();
        let number = |i: usize, min: f32, max: f32| -> Result<f32, String> {
            values[i]
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite() && *v >= min && *v <= max)
                .ok_or_else(|| format!("{} must be between {min} and {max}", modal.fields[i].0))
        };
        let result = (|| -> Result<(), String> {
            match modal.form {
                Form::Document => {
                    let size = [number(0, 16., 8192.)?, number(1, 16., 8192.)?];
                    let fps = number(2, 1., 120.)? as u32;
                    self.checkpoint();
                    self.stage_size = size;
                    self.fps = fps;
                }
                Form::Object => {
                    let dims = [
                        number(0, -100000., 100000.)?,
                        number(1, -100000., 100000.)?,
                        number(2, 1., 100000.)?,
                        number(3, 1., 100000.)?,
                    ];
                    let rotation = number(4, -36000., 36000.)?.to_radians();
                    let hex = values[5].trim().trim_start_matches('#');
                    if hex.len() != 6 {
                        return Err("Use a six-digit hex color".into());
                    }
                    let color = u32::from_str_radix(hex, 16).map_err(|_| "Invalid hex color")?;
                    if let Some(i) = self.editable_selection() {
                        self.checkpoint();
                        let o = &mut self.drawing.objects[i];
                        o.x = dims[0];
                        o.y = dims[1];
                        o.w = dims[2];
                        o.h = dims[3];
                        o.rotation = rotation;
                        o.fill = color;
                        if let workspace::Kind::Text(text) = &mut o.kind {
                            *text = values[6].clone();
                        }
                    }
                }
                Form::Rename => {
                    let name = values[0].trim();
                    if name.is_empty() {
                        return Err("Enter a layer name".into());
                    }
                    self.checkpoint();
                    self.layers[self.selected_layer].name = name.into();
                }
                Form::New => {
                    self.checkpoint();
                    self.layers = vec![LayerState::new("Layer 1", theme::ACCENT)];
                    self.drawing.objects.clear();
                    self.clear_selection();
                    self.drawing.drag = None;
                    self.text_editing = None;
                    self.symbols.clear();
                    self.selected_layer = 0;
                    self.set_layer_selection([0]);
                    self.layer_selection_anchor = Some(0);
                    self.clear_timeline_selection();
                    self.current_frame = 1;
                    self.frame_count = 32;
                    self.stage_size = [800., 450.];
                    self.fps = 24;
                    self.path = None;
                    self.saved.clear();
                    self.is_playing = false;
                    self.playback_task = None;
                }
                Form::Help => {}
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.status = "Changes applied".into();
                if let Some(f) = &self.focus {
                    window.focus(f, cx);
                }
            }
            Err(error) => {
                self.modal = Some(Modal { error, ..modal });
            }
        }
        cx.notify();
    }
}
