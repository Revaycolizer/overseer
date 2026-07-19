#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ActiveTab {
    Chat,
    Agents,
    Supervisors,
    Tools,
    Workflows,
    Browser,
    Computer,
    Settings,
    MCP,
    Skills,
    Metrics,
    SearchResults,
}

impl ActiveTab {
    pub const ALL: &'static [ActiveTab] = &[
        ActiveTab::Chat,
        ActiveTab::Agents,
        ActiveTab::Supervisors,
        ActiveTab::Tools,
        ActiveTab::Workflows,
        ActiveTab::Browser,
        ActiveTab::Computer,
        ActiveTab::Settings,
        ActiveTab::MCP,
        ActiveTab::Skills,
        ActiveTab::Metrics,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ActiveTab::Chat => "Chat",
            ActiveTab::Agents => "Agents",
            ActiveTab::Supervisors => "Supervisors",
            ActiveTab::Tools => "Tools",
            ActiveTab::Workflows => "Workflows",
            ActiveTab::Browser => "Browser",
            ActiveTab::Computer => "Computer",
            ActiveTab::Settings => "Settings",
            ActiveTab::MCP => "MCP",
            ActiveTab::Skills => "Skills",
            ActiveTab::Metrics => "Metrics",
            ActiveTab::SearchResults => "Search",
        }
    }

    pub fn index(&self) -> usize {
        Self::ALL.iter().position(|t| t == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Self {
        Self::ALL.get(i).copied().unwrap_or(ActiveTab::Chat)
    }

    pub fn next(&self) -> Self {
        let i = self.index();
        Self::from_index((i + 1) % Self::ALL.len())
    }

    pub fn prev(&self) -> Self {
        let len = Self::ALL.len();
        let i = self.index();
        Self::from_index((i + len - 1) % len)
    }
}
