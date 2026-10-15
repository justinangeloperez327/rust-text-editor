use gpui::{
    App, Application, Context, FocusHandle, IntoElement, Render, Window, WindowOptions, div,
    prelude::*, rgb,
};
use ide_core::layout::{BottomPanelTab, PanelDock, ResizeRegion, SidebarView, WorkspaceLayout};
use ide_document::{Document, cursor::Cursor};

#[derive(Clone, Copy)]
enum ResizeTarget {
    Sidebar,
    Panel,
}

struct EditorShell {
    document: Document,
    cursor: Cursor,
    editor_focus: FocusHandle,
    document_title: String,
    layout: WorkspaceLayout,
    settings_path: Option<std::path::PathBuf>,
    drag: Option<(ResizeTarget, f32)>,
}

impl EditorShell {
    fn key_down(&mut self, event: &gpui::KeyDownEvent, cx: &mut Context<Self>) {
        let stroke = &event.keystroke;
        if stroke.modifiers.control || stroke.modifiers.alt || stroke.modifiers.platform {
            return;
        }
        let result = match stroke.key.as_str() {
            "backspace" => {
                if self.cursor.has_selection() {
                    self.document.insert_at_cursor(&mut self.cursor, "")
                } else if self.cursor.head > 0 {
                    let end = self.cursor.head;
                    self.document.delete((end - 1)..end).map(|()| {
                        self.cursor.collapse_to(end - 1);
                    })
                } else {
                    return;
                }
            }
            "enter" => self.document.insert_at_cursor(&mut self.cursor, "\n"),
            "left" => {
                self.document.move_cursor_horizontal(
                    &mut self.cursor,
                    false,
                    stroke.modifiers.shift,
                );
                cx.notify();
                return;
            }
            "right" => {
                self.document.move_cursor_horizontal(
                    &mut self.cursor,
                    true,
                    stroke.modifiers.shift,
                );
                cx.notify();
                return;
            }
            "up" | "down" => {
                if let Err(error) = self.document.move_cursor_vertical(
                    &mut self.cursor,
                    stroke.key == "down",
                    stroke.modifiers.shift,
                ) {
                    tracing::warn!(?error, "cursor movement failed");
                }
                cx.notify();
                return;
            }
            "home" | "end" => {
                if let Err(error) = self.document.move_cursor_line_boundary(
                    &mut self.cursor,
                    stroke.key == "end",
                    stroke.modifiers.shift,
                ) {
                    tracing::warn!(?error, "cursor movement failed");
                }
                cx.notify();
                return;
            }
            "space" => self.document.insert_at_cursor(&mut self.cursor, " "),
            key if key.chars().count() == 1 => {
                let text = if stroke.modifiers.shift {
                    key.to_uppercase()
                } else {
                    key.to_owned()
                };
                self.document.insert_at_cursor(&mut self.cursor, &text)
            }
            _ => return,
        };
        if let Err(error) = result {
            tracing::warn!(?error, "document input failed");
        }
        cx.notify();
    }

    fn update_drag(&mut self, x: f32, y: f32, cx: &mut Context<Self>) {
        let Some((target, previous)) = self.drag else {
            return;
        };
        let current = match target {
            ResizeTarget::Sidebar => x,
            ResizeTarget::Panel if self.layout.panel_dock == PanelDock::Bottom => y,
            ResizeTarget::Panel => x,
        };
        let difference = current - previous;
        let region = match target {
            ResizeTarget::Sidebar => ResizeRegion::Sidebar,
            ResizeTarget::Panel => ResizeRegion::Panel,
        };
        if self.layout.resize_by(region, difference) {
            self.drag = Some((target, current));
            cx.notify();
        }
    }

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
        settings.panel_dock = self.layout.panel_dock.key().to_owned();
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
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                this.update_drag(event.position.x.into(), event.position.y.into(), cx);
            }))
            .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, _| {
                if this.drag.take().is_some() {
                    this.save_layout();
                }
            }))
            .child(
                div()
                    .h(gpui::px(34.0))
                    .flex()
                    .items_center()
                    .px_3()
                    .bg(rgb(0x181818))
                    .child("Rust Text Editor    File   Edit   Selection   View   Go   Run   Terminal   Help")
                    .child(
                        div()
                            .id("dock-bottom")
                            .cursor_pointer()
                            .ml_auto()
                            .child("Bottom")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.layout.set_panel_dock(PanelDock::Bottom);
                                this.save_layout();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("dock-left")
                            .cursor_pointer()
                            .ml_3()
                            .child("Left")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.layout.set_panel_dock(PanelDock::Left);
                                this.save_layout();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("dock-right")
                            .cursor_pointer()
                            .ml_3()
                            .child("Right")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.layout.set_panel_dock(PanelDock::Right);
                                this.save_layout();
                                cx.notify();
                            })),
                    ),
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
                    .when(self.layout.panel_visible && self.layout.panel_dock == PanelDock::Left, |this| {
                        this.child(
                            div()
                                .flex()
                                .child(
                                    div()
                                            .w(gpui::px(f32::from(self.layout.panel_height)))
                                            .flex()
                                            .flex_col()
                                            .bg(rgb(0x181818))
                                            .child(
                                                div()
                                                    .h(gpui::px(36.0))
                                                    .flex()
                                                    .items_center()
                                                    .px_3()
                                                    .child(self.layout.active_panel_tab.title()),
                                            )
                                            .child(
                                                div()
                                                    .p_3()
                                                    .text_color(rgb(0x999999))
                                                    .child(self.layout.active_panel_tab.empty_message()),
                                            ),
                                )
                                .child(
                                    div()
                                            .id("left-panel-resize-handle")
                                            .w(gpui::px(5.0))
                                            .bg(rgb(0x303030))
                                            .on_mouse_down(
                                                gpui::MouseButton::Left,
                                                cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                                    this.drag = Some((ResizeTarget::Panel, event.position.x.into()));
                                                }),
                                            ),
                                ),
                        )
                    })
                    .when(self.layout.sidebar_visible, |this| {
                        this.child(
                            div()
                                .id("sidebar-resize-handle")
                                .w(gpui::px(5.0))
                                .bg(rgb(0x303030))
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                        this.drag = Some((ResizeTarget::Sidebar, event.position.x.into()));
                                    }),
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
                                    .child(self.document_title.clone())
                                    .child(
                                        div()
                                            .id("document-append-newline")
                                            .ml_auto()
                                            .cursor_pointer()
                                            .child("Insert line")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if let Err(error) = this.document.insert(this.document.len_chars(), "\n") {
                                                    tracing::warn!(?error, "failed to insert text");
                                                }
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("document-undo")
                                            .ml_3()
                                            .cursor_pointer()
                                            .child("Undo")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.document.undo();
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .p_3()
                                    .font_family("Consolas")
                                    .overflow_y_scroll()
                                    .key_context("DocumentEditor")
                                    .track_focus(&self.editor_focus)
                                    .on_click(cx.listener(|this, _, window, _| {
                                        window.focus(&this.editor_focus);
                                    }))
                                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                                        this.key_down(event, cx);
                                    }))
                                    .child(self.document.text()),
                            )
                            .when(self.layout.panel_visible && self.layout.panel_dock == PanelDock::Bottom, |this| {
                                this.child(
                                    div()
                                        .id("bottom-resize-handle")
                                        .h(gpui::px(5.0))
                                        .bg(rgb(0x303030))
                                        .on_mouse_down(
                                            gpui::MouseButton::Left,
                                            cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                                this.drag = Some((ResizeTarget::Panel, event.position.y.into()));
                                            }),
                                        ),
                                )
                                .child(
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
                    )
                    .when(self.layout.panel_visible && self.layout.panel_dock == PanelDock::Right, |this| {
                        this.child(
                            div()
                                .flex()
                                .child(
                                    div()
                                            .id("right-panel-resize-handle")
                                            .w(gpui::px(5.0))
                                            .bg(rgb(0x303030))
                                            .on_mouse_down(
                                                gpui::MouseButton::Left,
                                                cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                                    this.drag = Some((ResizeTarget::Panel, event.position.x.into()));
                                                }),
                                            ),
                                )
                                .child(
                                    div()
                                            .w(gpui::px(f32::from(self.layout.panel_height)))
                                            .flex()
                                            .flex_col()
                                            .bg(rgb(0x181818))
                                            .child(
                                                div()
                                                    .h(gpui::px(36.0))
                                                    .flex()
                                                    .items_center()
                                                    .px_3()
                                                    .child(self.layout.active_panel_tab.title()),
                                            )
                                            .child(
                                                div()
                                                    .p_3()
                                                    .text_color(rgb(0x999999))
                                                    .child(self.layout.active_panel_tab.empty_message()),
                                            ),
                                ),
                        )
                    }),
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
                        .child(format!("  Cursor {} / {}", self.cursor.head, self.document.len_chars()))
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
        panel_dock: PanelDock::from_key(&settings.panel_dock).unwrap_or_default(),
    };

    let (document_title, document) = match std::env::args_os().nth(1) {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            match Document::open(&path) {
                Ok(document) => (path.display().to_string(), document),
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "could not open requested file");
                    ("Untitled".to_owned(), Document::new(""))
                }
            }
        }
        None => ("Untitled".to_owned(), Document::new("")),
    };

    tracing::info!(app = ide_core::APPLICATION_NAME, "starting native IDE");
    Application::new().run(move |cx: &mut App| {
        cx.open_window(WindowOptions::default(), move |_window, cx| {
            cx.new(move |cx| EditorShell {
                cursor: Cursor::new(document.len_chars()),
                editor_focus: cx.focus_handle(),
                document,
                document_title,
                layout,
                settings_path,
                drag: None,
            })
        })
        .expect("could not open the editor window");
        cx.activate(true);
    });
}
