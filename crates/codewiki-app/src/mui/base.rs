//! What every pressable control shares: MUI's `ButtonBase`.
//!
//! A control built on [`Base`] knows whether the pointer is over it, whether
//! it is pressed and whether it has keyboard focus; it draws a ripple when
//! pressed; and Tab reaches it and Enter or Space press it.
//!
//! GPUI switches `.hover()` and `.active()` styles instantly. So that a
//! control can ease between its states, the state is kept here and the
//! control reads it while it renders.

use std::rc::Rc;
use std::time::Instant;

use gpui_kit::{
    App, BoxShadow, ClickEvent, Div, ElementId, Entity, FocusHandle, Hsla, InteractiveElement as _,
    MouseButton, ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled as _,
    Window, canvas, point, px,
};

use super::ripple::{self, Wave};

/// What a click does.
pub type OnClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The remembered state of one control.
pub struct Touch {
    hovered: bool,
    pressed: bool,
    waves: Vec<Wave>,
    focus: FocusHandle,
}

impl Touch {
    fn press(&mut self, at: Option<ripple::Point>) {
        self.pressed = true;
        self.waves.push(Wave::new(at));
    }

    fn release(&mut self) {
        self.pressed = false;
        for wave in &mut self.waves {
            wave.release();
        }
    }
}

/// How a control's ripple is drawn.
#[derive(Clone, Copy)]
pub struct Ink {
    /// Corner radius of the control, in pixels.
    pub radius: f32,
    /// Start from the middle, as on an icon button.
    pub centered: bool,
    /// MUI's `currentColor`: the color of the control's text.
    pub color: Hsla,
}

/// The state of one control for this frame, and the means to wire it up.
pub struct Base {
    state: Entity<Touch>,
    pub hovered: bool,
    pub pressed: bool,
    /// Keyboard focus. A press with the mouse does not give focus.
    pub focused: bool,
}

impl Base {
    /// The state of the control `id`. Call while rendering it.
    pub fn new(id: &ElementId, disabled: bool, window: &mut Window, cx: &mut App) -> Self {
        let state = window.use_keyed_state(id.clone(), cx, |_, cx| Touch {
            hovered: false,
            pressed: false,
            waves: Vec::new(),
            focus: cx.focus_handle(),
        });
        let now = Instant::now();
        let moving = state.update(cx, |touch, _| {
            if disabled {
                touch.hovered = false;
                touch.release();
            }
            touch.waves.retain(|wave| !wave.finished(now));
            // A ripple held at full size has nothing left to animate.
            touch.waves.iter().any(|wave| wave.moving(now))
        });
        if moving && !cx.reduce_motion() {
            window.request_animation_frame();
        }
        let touch = state.read(cx);
        Self {
            hovered: touch.hovered,
            pressed: touch.pressed,
            focused: !disabled && touch.focus.is_focused(window),
            state,
        }
    }

    /// Make `element` the control: pointer and keyboard handling, and the
    /// ripple drawn over its content. A disabled control gets none of it.
    pub fn attach(
        &self,
        element: Stateful<Div>,
        ink: Ink,
        disabled: bool,
        on_click: Option<OnClick>,
        cx: &App,
    ) -> Stateful<Div> {
        if disabled {
            return element;
        }
        let focus = self.state.read(cx).focus.clone();
        let reduce = cx.reduce_motion();
        let (hover, down, up, up_out, click, paint) = (
            self.state.clone(),
            self.state.clone(),
            self.state.clone(),
            self.state.clone(),
            self.state.clone(),
            self.state.clone(),
        );
        element
            .track_focus(&focus)
            .tab_index(0)
            .tab_stop(true)
            .cursor_pointer()
            .on_hover(move |hovered, _, cx| {
                hover.update(cx, |touch, cx| {
                    touch.hovered = *hovered;
                    if !*hovered {
                        touch.release();
                    }
                    cx.notify();
                });
            })
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                // A press with the mouse must not move keyboard focus.
                window.prevent_default();
                let at = (event.position.x / px(1.), event.position.y / px(1.));
                down.update(cx, |touch, cx| {
                    touch.press(Some(at));
                    cx.notify();
                });
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                up.update(cx, |touch, cx| {
                    touch.release();
                    cx.notify();
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                up_out.update(cx, |touch, cx| {
                    if touch.pressed {
                        touch.release();
                        cx.notify();
                    }
                });
            })
            .on_click(move |event, window, cx| {
                if matches!(event, ClickEvent::Keyboard(_)) {
                    // A key press ripples from the middle and lets go at once.
                    click.update(cx, |touch, cx| {
                        touch.press(None);
                        touch.release();
                        cx.notify();
                    });
                }
                if let Some(on_click) = &on_click {
                    on_click(event, window, cx);
                }
            })
            .child(
                canvas(
                    |bounds, _, _| bounds,
                    move |_, bounds, window, cx| {
                        let touch = paint.read(cx);
                        if !reduce {
                            ripple::paint(
                                bounds,
                                ink.radius,
                                &touch.waves,
                                ink.centered,
                                ink.color,
                                window,
                            );
                        } else if touch.pressed {
                            // Without motion a press is a plain tint.
                            ripple::paint_pressed(bounds, ink.radius, ink.color, window);
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
    }
}

/// The ring a control shows while it has keyboard focus. TradingAgent's
/// `focusRing`: `0 0 0 2px rgba(primary, 0.4)`.
pub fn focus_ring(primary: Hsla) -> BoxShadow {
    BoxShadow {
        color: primary.opacity(0.4),
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: px(2.),
        inset: false,
    }
}
