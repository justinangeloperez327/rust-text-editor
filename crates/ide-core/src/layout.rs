//! UI-independent workspace layout and docking preferences.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarView {
    #[default]
    Explorer,
    Search,
    SourceControl,
    Run,
    Extensions,
}

impl SidebarView {
    pub const ALL: [Self; 5] = [
        Self::Explorer,
        Self::Search,
        Self::SourceControl,
        Self::Run,
        Self::Extensions,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Explorer => "EXPLORER",
            Self::Search => "SEARCH",
            Self::SourceControl => "SOURCE CONTROL",
            Self::Run => "RUN AND DEBUG",
            Self::Extensions => "EXTENSIONS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BottomPanelTab {
    Problems,
    Output,
    DebugConsole,
    #[default]
    Terminal,
}

impl BottomPanelTab {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Problems => "PROBLEMS",
            Self::Output => "OUTPUT",
            Self::DebugConsole => "DEBUG CONSOLE",
            Self::Terminal => "TERMINAL",
        }
    }

    pub const fn empty_message(self) -> &'static str {
        match self {
            Self::Problems => "No problems detected",
            Self::Output => "No output available",
            Self::DebugConsole => "No active debug session",
            Self::Terminal => "No active terminal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceLayout {
    pub sidebar_view: SidebarView,
    pub active_panel_tab: BottomPanelTab,
    pub sidebar_visible: bool,
    pub panel_visible: bool,
    pub status_bar_visible: bool,
    pub sidebar_width: u16,
    pub panel_height: u16,
}

impl Default for WorkspaceLayout {
    fn default() -> Self {
        Self {
            sidebar_view: SidebarView::Explorer,
            active_panel_tab: BottomPanelTab::Terminal,
            sidebar_visible: true,
            panel_visible: true,
            status_bar_visible: true,
            sidebar_width: 240,
            panel_height: 180,
        }
    }
}

impl WorkspaceLayout {
    pub fn set_sidebar_width(&mut self, width: u16) {
        self.sidebar_width = width.clamp(160, 600);
    }

    pub fn set_panel_height(&mut self, height: u16) {
        self.panel_height = height.clamp(100, 480);
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    pub fn select_panel_tab(&mut self, tab: BottomPanelTab) {
        self.active_panel_tab = tab;
        self.panel_visible = true;
    }

    pub fn toggle_panel(&mut self) {
        self.panel_visible = !self.panel_visible;
    }

    pub fn select_view(&mut self, view: SidebarView) {
        if self.sidebar_view == view && self.sidebar_visible {
            self.sidebar_visible = false;
        } else {
            self.sidebar_view = view;
            self.sidebar_visible = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_has_expected_defaults() {
        let layout = WorkspaceLayout::default();
        assert_eq!(layout.sidebar_width, 240);
        assert_eq!(layout.panel_height, 180);
        assert!(layout.panel_visible);
    }

    #[test]
    fn dimensions_are_clamped() {
        let mut layout = WorkspaceLayout::default();
        layout.set_sidebar_width(20);
        layout.set_panel_height(2000);
        assert_eq!(layout.sidebar_width, 160);
        assert_eq!(layout.panel_height, 480);
    }

    #[test]
    fn selecting_panel_tab_opens_panel() {
        let mut layout = WorkspaceLayout::default();
        layout.panel_visible = false;
        layout.select_panel_tab(BottomPanelTab::Problems);
        assert!(layout.panel_visible);
        assert_eq!(layout.active_panel_tab, BottomPanelTab::Problems);
        assert_eq!(layout.active_panel_tab.empty_message(), "No problems detected");
    }

    #[test]
    fn selecting_same_view_collapses_sidebar() {
        let mut layout = WorkspaceLayout::default();
        layout.select_view(SidebarView::Explorer);
        assert!(!layout.sidebar_visible);
        layout.select_view(SidebarView::Search);
        assert_eq!(layout.sidebar_view, SidebarView::Search);
        assert!(layout.sidebar_visible);
    }
}
