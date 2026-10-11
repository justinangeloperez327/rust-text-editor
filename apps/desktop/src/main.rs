use gpui::{App, Application, Context, IntoElement, Render, Window, WindowOptions, div, prelude::*, rgb};

struct EditorShell;

impl Render for EditorShell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xd4d4d4))
            .child(
                div()
                    .h(gpui::px(36.0))
                    .flex()
                    .items_center()
                    .px_3()
                    .bg(rgb(0x252526))
                    .child("Rust Text Editor"),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .child(
                        div()
                            .w(gpui::px(220.0))
                            .p_3()
                            .bg(rgb(0x252526))
                            .child("EXPLORER"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("Welcome to Rust Text Editor"),
                    ),
            )
            .child(
                div()
                    .h(gpui::px(24.0))
                    .flex()
                    .items_center()
                    .px_3()
                    .bg(rgb(0x007acc))
                    .child("Rust IDE  ·  Group 1"),
            )
    }
}

fn main() {
    let _app_name = ide_core::APPLICATION_NAME;
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|_| EditorShell)
        })
        .expect("could not open the editor window");
        cx.activate(true);
    });
}
