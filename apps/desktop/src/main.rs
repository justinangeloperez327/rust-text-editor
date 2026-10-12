use gpui::{
    App, Application, Context, IntoElement, Render, Window, WindowOptions, div, prelude::*, rgb,
};
use ide_core::layout::WorkspaceLayout;

struct EditorShell {
    layout: WorkspaceLayout,
}

impl Render for EditorShell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xd4d4d4))
            .text_sm()
            .child(
                div()
                    .h(gpui::px(34.0))
                    .flex()
                    .items_center()
                    .px_3()
                    .bg(rgb(0x181818))
                    .child("Rust Text Editor    File   Edit   Selection   View   Go   Run   Terminal   Help"),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .child(
                        div()
                            .w(gpui::px(48.0))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_4()
                            .py_4()
                            .bg(rgb(0x181818))
                            .child("F")
                            .child("S")
                            .child("G")
                            .child("R")
                            .child("X"),
                    )
                    .when(self.layout.sidebar_visible, |this| {
                        this.child(
                            div()
                                .w(gpui::px(f32::from(self.layout.sidebar_width)))
                                .flex()
                                .flex_col()
                                .bg(rgb(0x252526))
                                .child(
                                    div()
                                        .h(gpui::px(36.0))
                                        .flex()
                                        .items_center()
                                        .px_3()
                                        .child(self.layout.sidebar_view.title()),
                                )
                                .child(
                                    div()
                                        .px_3()
                                        .py_2()
                                        .text_color(rgb(0x999999))
                                        .child("No folder opened"),
                                ),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .h(gpui::px(36.0))
                                    .flex()
                                    .items_center()
                                    .px_3()
                                    .bg(rgb(0x252526))
                                    .child("Welcome"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child("Open a folder or file to get started"),
                            )
                            .when(self.layout.panel_visible, |this| {
                                this.child(
                                    div()
                                        .h(gpui::px(f32::from(self.layout.panel_height)))
                                        .flex()
                                        .flex_col()
                                        .bg(rgb(0x181818))
                                        .child(
                                            div()
                                                .h(gpui::px(36.0))
                                                .flex()
                                                .items_center()
                                                .px_3()
                                                .child("PROBLEMS      OUTPUT      DEBUG CONSOLE      TERMINAL"),
                                        )
                                        .child(
                                            div()
                                                .px_3()
                                                .py_2()
                                                .text_color(rgb(0x999999))
                                                .child("No active terminal"),
                                        ),
                                )
                            }),
                    ),
            )
            .when(self.layout.status_bar_visible, |this| {
                this.child(
                    div()
                        .h(gpui::px(24.0))
                        .flex()
                        .items_center()
                        .px_3()
                        .bg(rgb(0x007acc))
                        .child("main     UTF-8      Rust Text Editor"),
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
    let mut layout = WorkspaceLayout::default();
    layout.sidebar_visible = settings.show_sidebar;
    layout.status_bar_visible = settings.show_status_bar;

    tracing::info!(app = ide_core::APPLICATION_NAME, "starting native IDE");
    Application::new().run(move |cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|_| EditorShell { layout })
        })
        .expect("could not open the editor window");
        cx.activate(true);
    });
}
