use gpui::{
    App, Application, Context, FocusHandle, IntoElement, Render, Window, WindowOptions, div,
    prelude::*, rgb,
};
use ide_core::layout::{BottomPanelTab, PanelDock, ResizeRegion, SidebarView, WorkspaceLayout};
use ide_document::{
    Document,
    cursor::{Cursor, TextPosition},
};

#[derive(Clone, Copy)]
enum ResizeTarget {
    Sidebar,
    Panel,
}

struct EditorShell {
    document: Document,
    cursor: Cursor,
    cursor_undo: Vec<(Cursor, Cursor)>,
    cursor_redo: Vec<(Cursor, Cursor)>,
    editor_focus: FocusHandle,
    document_title: String,
    document_path: Option<std::path::PathBuf>,
    viewport_start_line: usize,
    viewport_page_lines: usize,
    layout: WorkspaceLayout,
    settings_path: Option<std::path::PathBuf>,
    drag: Option<(ResizeTarget, f32)>,
}

impl EditorShell {
    fn key_down(&mut self, event: &gpui::KeyDownEvent, cx: &mut Context<Self>) {
        let before_version = self.document.version();
        let before_cursor = self.cursor;
        self.handle_key_down(event, cx);
        let shortcut = &event.keystroke;
        let undo_or_redo = (shortcut.modifiers.control || shortcut.modifiers.platform)
            && (shortcut.key == "z" || shortcut.key == "y");
        if !undo_or_redo && self.document.version() != before_version {
            self.cursor_undo.push((before_cursor, self.cursor));
            self.cursor_redo.clear();
        }
    }

    fn handle_key_down(&mut self, event: &gpui::KeyDownEvent, cx: &mut Context<Self>) {
        let stroke = &event.keystroke;
        if (stroke.modifiers.control || stroke.modifiers.platform)
            && (stroke.key == "z" || stroke.key == "y")
        {
            let redo = stroke.key == "y" || stroke.modifiers.shift;
            let changed = if redo {
                self.document.redo()
            } else {
                self.document.undo()
            };
            if changed {
                let transition = if redo {
                    self.cursor_redo.pop().map(|pair| {
                        self.cursor_undo.push(pair);
                        pair.1
                    })
                } else {
                    self.cursor_undo.pop().map(|pair| {
                        self.cursor_redo.push(pair);
                        pair.0
                    })
                };
                if let Some(cursor) = transition {
                    self.cursor = cursor;
                } else {
                    self.cursor
                        .collapse_to(self.cursor.head.min(self.document.len_chars()));
                }
                self.keep_cursor_in_view();
                cx.notify();
            }
            return;
        }
        if (stroke.modifiers.control || stroke.modifiers.platform)
            && (stroke.key == "c" || stroke.key == "x" || stroke.key == "v")
        {
            if stroke.key == "v" {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
                    && let Err(error) = self.document.insert_at_cursor(&mut self.cursor, &text)
                {
                    tracing::warn!(?error, "clipboard paste failed");
                }
            } else if self.cursor.has_selection() {
                let range = self.cursor.selection();
                let text = self.document.text();
                let selected: String = text
                    .chars()
                    .skip(range.start)
                    .take(range.end - range.start)
                    .collect();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(selected));
                if stroke.key == "x"
                    && let Err(error) = self.document.insert_at_cursor(&mut self.cursor, "")
                {
                    tracing::warn!(?error, "clipboard cut failed");
                }
            }
            self.keep_cursor_in_view();
            cx.notify();
            return;
        }
        if (stroke.modifiers.control || stroke.modifiers.platform)
            && stroke.key == "backspace"
        {
            if self.cursor.has_selection() {
                if let Err(error) = self.document.insert_at_cursor(&mut self.cursor, "") {
                    tracing::warn!(?error, "delete selection failed");
                }
            } else if self.cursor.head > 0 {
                let text = self.document.text();
                let chars: Vec<char> = text.chars().take(self.cursor.head).collect();
                let mut start = chars.len();
                while start > 0 && chars[start - 1].is_whitespace() {
                    start -= 1;
                }
                while start > 0 && !chars[start - 1].is_whitespace() {
                    start -= 1;
                }
                if let Err(error) = self.document.delete(start..self.cursor.head) {
                    tracing::warn!(?error, "delete previous word failed");
                } else {
                    self.cursor.collapse_to(start);
                }
            }
            self.keep_cursor_in_view();
            cx.notify();
            return;
        }
        if (stroke.modifiers.control || stroke.modifiers.platform) && stroke.key == "a" {
            self.cursor.anchor = 0;
            self.cursor.head = self.document.len_chars();
            self.keep_cursor_in_view();
            cx.notify();
            return;
        }
        if (stroke.modifiers.control || stroke.modifiers.platform) && stroke.key == "s" {
            if let Some(path) = self.document_path.as_deref()
                && let Err(error) = self.document.save(path)
            {
                tracing::warn!(%error, "failed to save document");
            }
            cx.notify();
            return;
        }
        if (stroke.modifiers.control || stroke.modifiers.platform)
            && (stroke.key == "home" || stroke.key == "end")
        {
            self.move_to_document_boundary(stroke.key == "end", stroke.modifiers.shift);
            cx.notify();
            return;
        }
        if stroke.modifiers.control || stroke.modifiers.alt || stroke.modifiers.platform {
            return;
        }
        let result = match stroke.key.as_str() {
            "delete" => {
                if self.cursor.has_selection() {
                    self.document.insert_at_cursor(&mut self.cursor, "")
                } else if self.cursor.head < self.document.len_chars() {
                    self.document.delete(self.cursor.head..self.cursor.head + 1)
                } else {
                    return;
                }
            }
            "tab" => self.document.insert_at_cursor(&mut self.cursor, "    "),
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
                self.keep_cursor_in_view();
                cx.notify();
                return;
            }
            "right" => {
                self.document.move_cursor_horizontal(
                    &mut self.cursor,
                    true,
                    stroke.modifiers.shift,
                );
                self.keep_cursor_in_view();
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
                self.keep_cursor_in_view();
                cx.notify();
                return;
            }
            "escape" => {
                self.cursor.collapse_to(self.cursor.head);
                cx.notify();
                return;
            }
            "pageup" | "pagedown" => {
                let down = stroke.key == "pagedown";
                for _ in 0..self.viewport_page_lines {
                    if let Err(error) = self.document.move_cursor_vertical(
                        &mut self.cursor,
                        down,
                        stroke.modifiers.shift,
                    ) {
                        tracing::warn!(?error, "page navigation failed");
                        break;
                    }
                }
                self.keep_cursor_in_view();
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
                self.keep_cursor_in_view();
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
        self.keep_cursor_in_view();
        cx.notify();
    }

    fn move_to_document_boundary(&mut self, end: bool, extend_selection: bool) {
        let offset = if end { self.document.len_chars() } else { 0 };
        if extend_selection {
            self.cursor.extend_to(offset);
        } else {
            self.cursor.collapse_to(offset);
        }
        self.keep_cursor_in_view();
    }

    fn keep_cursor_in_view(&mut self) {
        if let Ok(position) = self.document.position_at(self.cursor.head) {
            if position.line < self.viewport_start_line {
                self.viewport_start_line = position.line;
            } else if position.line >= self.viewport_start_line.saturating_add(200) {
                self.viewport_start_line = position.line.saturating_sub(199);
            }
        }
    }

    fn scroll_lines(&mut self, lines: isize, cx: &mut Context<Self>) {
        let maximum = self.document.len_lines().saturating_sub(1);
        self.viewport_start_line = self
            .viewport_start_line
            .saturating_add_signed(lines)
            .min(maximum);
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
                                                if this.document.undo() {
                                                    if let Some(pair) = this.cursor_undo.pop() {
                                                        this.cursor_redo.push(pair);
                                                        this.cursor = pair.0;
                                                    } else {
                                                        this.cursor.collapse_to(this.cursor.head.min(this.document.len_chars()));
                                                    }
                                                    this.keep_cursor_in_view();
                                                }
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .p_3()
                                    .font_family("Consolas")
                                    .id("document-editor")
                                    .key_context("DocumentEditor")
                                    .track_focus(&self.editor_focus)
                                    .on_click(cx.listener(|this, _, window, _| {
                                        window.focus(&this.editor_focus);
                                    }))
                                    .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, cx| {
                                        let delta = event.delta.pixel_delta(gpui::px(18.0));
                                        let lines = (f32::from(delta.y) / 18.0).round() as isize;
                                        if lines != 0 {
                                            this.scroll_lines(-lines, cx);
                                        }
                                    }))
                                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                                        this.key_down(event, cx);
                                    }))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child({
                                                let caret = self.document.position_at(self.cursor.head).ok();
                                                let selection = self.cursor.selection();
                                                let mut lines = div().flex().flex_col();
                                                let mut line_offset = self.document.offset_at(TextPosition {
                                                    line: self.viewport_start_line,
                                                    column: 0,
                                                }).unwrap_or(0);
                                                let viewport_end = self.viewport_start_line.saturating_add(200).min(self.document.len_lines());
                                                for index in self.viewport_start_line..viewport_end {
                                                    let raw_line = self.document.line(index).unwrap_or_default();
                                                    let line = raw_line.trim_end_matches(['\r', '\n']);
                                                    let mut row = div().flex().items_center();
                                                    let mut segment = String::new();
                                                    let mut highlighted = false;
                                                    for (column, character) in line.chars().enumerate() {
                                                        if caret.is_some_and(|position| position.line == index && position.column == column) {
                                                            if !segment.is_empty() {
                                                                row = row.child(div().when(highlighted, |element| element.bg(rgb(0x264f78))).child(std::mem::take(&mut segment)));
                                                            }
                                                            row = row.child(div().w(gpui::px(2.0)).h(gpui::px(16.0)).bg(rgb(0xd4d4d4)));
                                                        }
                                                        let selected = selection.contains(&(line_offset + column));
                                                        if selected != highlighted && !segment.is_empty() {
                                                            row = row.child(div().when(highlighted, |element| element.bg(rgb(0x264f78))).child(std::mem::take(&mut segment)));
                                                        }
                                                        highlighted = selected;
                                                        segment.push(character);
                                                    }
                                                    if !segment.is_empty() {
                                                        row = row.child(div().when(highlighted, |element| element.bg(rgb(0x264f78))).child(segment));
                                                    }
                                                    if caret.is_some_and(|position| position.line == index && position.column >= line.chars().count()) {
                                                        row = row.child(div().w(gpui::px(2.0)).h(gpui::px(16.0)).bg(rgb(0xd4d4d4)));
                                                    }
                                                    lines = lines.child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .when(caret.is_some_and(|position| position.line == index), |element| element.bg(rgb(0x252526)))
                                                            .child(
                                                                div()
                                                                    .w(gpui::px(52.0))
                                                                    .pr_3()
                                                                    .text_color(rgb(0x858585))
                                                                    .child(format!("{}", index + 1)),
                                                            )
                                                            .child(row.min_h(gpui::px(18.0))),
                                                    );
                                                    line_offset += raw_line.chars().count();
                                                }
                                                lines
                                            })
                                            .child(
                                                div()
                                                    .text_color(rgb(0x4ec9b0))
                                                    .child(format!(
                                                        "Position: {}",
                                                        self.document.position_at(self.cursor.head).map_or_else(
                                                            |_| "unknown".to_owned(),
                                                            |position| format!("{}:{}", position.line + 1, position.column + 1)
                                                        )
                                                    )),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .mt_3()
                                            .text_color(rgb(0x569cd6))
                                            .child(format!(
                                                "Caret: {} | Selection: {}..{}",
                                                self.cursor.head,
                                                self.cursor.selection().start,
                                                self.cursor.selection().end
                                            )),
                                    ),
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

    let (document_title, document_path, document) = match std::env::args_os().nth(1) {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            match Document::open(&path) {
                Ok(document) => (path.display().to_string(), Some(path), document),
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "could not open requested file");
                    ("Untitled".to_owned(), None, Document::new(""))
                }
            }
        }
        None => ("Untitled".to_owned(), None, Document::new("")),
    };

    tracing::info!(app = ide_core::APPLICATION_NAME, "starting native IDE");
    Application::new().run(move |cx: &mut App| {
        cx.open_window(WindowOptions::default(), move |_window, cx| {
            cx.new(move |cx| EditorShell {
                cursor: Cursor::new(document.len_chars()),
                cursor_undo: Vec::new(),
                cursor_redo: Vec::new(),
                editor_focus: cx.focus_handle(),
                document,
                document_title,
                document_path,
                viewport_start_line: 0,
                viewport_page_lines: 40,
                layout,
                settings_path,
                drag: None,
            })
        })
        .expect("could not open the editor window");
        cx.activate(true);
    });
}
