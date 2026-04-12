/// Where a note was loaded from
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteSource {
    UserGlobal,
    RepoScoped,
}

/// A discovered note ready for display
#[derive(Debug, Clone)]
pub struct DiscoveredNote {
    pub topic: String,
    pub description: String,
    pub category: String,
    pub content: String,
    pub source: NoteSource,
}
