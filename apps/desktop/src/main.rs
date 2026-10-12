use gpui::{
    App, Application, Context, IntoElement, Render, Window, WindowOptions, div, prelude::*, rgb,
};
use ide_core::layout::{BottomPanelTab, SidebarView, WorkspaceLayout};

struct EditorShell {
    layout: WorkspaceLayout,
    settings_path: Option<std::path::PathBuf>,
}

impl EditorShell {
    fn save_layout(&self) {
        let Some(path) = self.settings_path.as_deref() else {
            return;
        };
        let mut settings = ide_settings::EditorSettings::load(path).unwrap_or_default();
        settings.show_sidebar = self.layout.sidebar_visible;
        settings.show_status_bar = self.layout.status_bar_visible;
        settings.show_panel = self.layout.panel_visible;
        settings.sidebar_width = self.layout.sidebar_width;
        settings.panel_height = self.layout.panel_height;
        settings.sidebar_view = self.layout.sidebar_view.key().to_owned();
        settings.active_panel_tab = self.layout.active_panel_tab.key().to_owned();
        if let Err(error) = settings.save(path) {
            tracing::warn!(%error, "could not persist workspace layout");
        }
    }
}

impl Render for EditorShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child(
                                div()
                                    .id("activity-explorer")
                                    .cursor_pointer()
                                    .child("F")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.layout.select_view(SidebarView::Explorer);
                                        this.save_layout();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("activity-search")
                                    .cursor_pointer()
                                    .child("S")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.layout.select_view(SidebarView::Search);
                                        this.save_layout();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("activity-source-control")
                                    .cursor_pointer()
                                    .child("G")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.layout.select_view(SidebarView::SourceControl);
                                        this.save_layout();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("activity-debug")
                                    .cursor_pointer()
                                    .child("R")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.layout.select_view(SidebarView::Run);
                                        this.save_layout();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("activity-extensions")
                                    .cursor_pointer()
                                    .child("X")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.layout.select_view(SidebarView::Extensions);
                                        this.save_layout();
                                        cx.notify();
                                    })),
                            ),
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
                                        .child(self.layout.sidebar_view.title())
                                        .child(
                                            div()
                                                .id("narrow-sidebar")
                                                .cursor_pointer()
                                                .ml_auto()
                                                .child("−")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.layout.set_sidebar_width(
                                                        this.layout.sidebar_width.saturating_sub(24),
                                                    );
                                                    this.save_layout();
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id("widen-sidebar")
                                                .cursor_pointer()
                                                .ml_3()
                                                .child("+")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.layout.set_sidebar_width(
                                                        this.layout.sidebar_width.saturating_add(24),
                                                    );
                                                    this.save_layout();
                                                    cx.notify();
                                                })),
                                        ),
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
                                                .child(
                                                    div()
                                                        .id("tab-problems")
                                                        .cursor_pointer()
                                                        .child("PROBLEMS")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.layout.select_panel_tab(BottomPanelTab::Problems);
                                                            this.save_layout();
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    div()
                                                        .id("tab-output")
                                                        .cursor_pointer()
                                                        .ml_3()
                                                        .child("OUTPUT")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.layout.select_panel_tab(BottomPanelTab::Output);
                                                            this.save_layout();
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    div()
                                                        .id("tab-debug-console")
                                                        .cursor_pointer()
                                                        .ml_3()
                                                        .child("DEBUG CONSOLE")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.layout.select_panel_tab(BottomPanelTab::DebugConsole);
                                                            this.save_layout();
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    div()
                                                        .id("tab-terminal")
                                                        .cursor_pointer()
                                                        .ml_3()
                                                        .child("TERMINAL")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.layout.select_panel_tab(BottomPanelTab::Terminal);
                                                            this.save_layout();
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    div()
                                                        .id("shrink-panel")
                                                        .cursor_pointer()
                                                        .ml_auto()
                                                        .child("−")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.layout.set_panel_height(
                                                                this.layout.panel_height.saturating_sub(24),
                                                            );
                                                            this.save_layout();
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    div()
                                                        .id("grow-panel")
                                                        .cursor_pointer()
                                                        .ml_3()
                                                        .child("+")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.layout.set_panel_height(
                                                                this.layout.panel_height.saturating_add(24),
                                                            );
                                                            this.save_layout();
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    div()
                                                        .id("hide-panel")
                                                        .cursor_pointer()
                                                        .ml_auto()
                                                        .child("×")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.layout.toggle_panel();
                                                            this.save_layout();
                                                            cx.notify();
                                                        })),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .px_3()
                                                .py_2()
                                                .text_color(rgb(0x999999))
                                                .child(self.layout.active_panel_tab.empty_message()),
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
                        .child("Rust Text Editor")
                        .child(
                            div()
                                .id("toggle-panel")
                                .cursor_pointer()
                                .ml_auto()
                                .child("Toggle Panel")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.layout.toggle_panel();
                                    this.save_layout();
                                    cx.notify();
                                })),
                        ),
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

    let settings_path = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .map(|path| path.join("RustTextEditor").join("settings.toml"));
    let settings = settings_path
        .as_deref()
        .and_then(|path| match ide_settings::EditorSettings::load(path) {
            Ok(settings) => Some(settings),
            Err(error) => {
                tracing::warn!(%error, "could not load user preferences; using defaults");
                None
            }
        })
        .unwrap_or_default();
    let layout = WorkspaceLayout {
        sidebar_visible: settings.show_sidebar,
        status_bar_visible: settings.show_status_bar,
        panel_visible: settings.show_panel,
        sidebar_width: settings.sidebar_width,
        panel_height: settings.panel_height,
        sidebar_view: SidebarView::from_key(&settings.sidebar_view).unwrap_or_default(),
        active_panel_tab: BottomPanelTab::from_key(&settings.active_panel_tab).unwrap_or_default(),
        ..WorkspaceLayout::default()
    };

    tracing::info!(app = ide_core::APPLICATION_NAME, "starting native IDE");
    Application::new().run(move |cx: &mut App| {
        cx.open_window(WindowOptions::default(), move |_window, cx| {
            cx.new(move |_| EditorShell {
                layout,
                settings_path,
            })
        })
        .expect("could not open the editor window");
        cx.activate(true);
    });
}
