use gpui::{
    App, Application, Context, IntoElement, Render, Window, WindowOptions, div, prelude::*, rgb,
};

struct EditorShell {
    show_sidebar: bool,
    show_status_bar: bool,
}

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
                    .when(self.show_sidebar, |this| {
                        this.child(
                            div()
                                .w(gpui::px(220.0))
                                .p_3()
                                .bg(rgb(0x252526))
                                .child("EXPLORER"),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("Welcome to Rust Text Editor"),
                    ),
            )
            .when(self.show_status_bar, |this| {
                this.child(
                    div()
                        .h(gpui::px(24.0))
                        .flex()
                        .items_center()
                        .px_3()
                        .bg(rgb(0x007acc))
                        .child("Rust IDE  ·  Group 2"),
                )
            })
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init()
        .ok();
    let mut lifecycle = ide_core::state::ApplicationState::default();
    lifecycle.started().expect("invalid startup transition");
    let settings = ide_settings::EditorSettings::default();
    tracing::info!(app = ide_core::APPLICATION_NAME, "starting native IDE");
    Application::new().run(move |cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|_| EditorShell {
                show_sidebar: settings.show_sidebar,
                show_status_bar: settings.show_status_bar,
            })
        })
        .expect("could not open the editor window");
        cx.activate(true);
    });
}
