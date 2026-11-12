//! Spike S1, step 1: prove `gpui-kit =0.9.1` builds and opens a window on this machine.
//! Example taken from the gpui-kit 0.9.1 crate-level documentation.

use gpui_kit::*;

struct Hello;

impl Render for Hello {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child("Hello, CodeWiki!")
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Hello))
            .expect("failed to open window");
    });
}
