use super::metadata::{DiscoveredNote, NoteSource};
use crate::error::{ForgeError, ForgeResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Discover notes from user-global and repo-scoped directories.
/// Repo-scoped notes override user-global ones by topic name.
pub fn discover_notes(
    user_dir: &Path,
    repo_dir: Option<&Path>,
) -> ForgeResult<Vec<DiscoveredNote>> {
    let mut notes: HashMap<String, DiscoveredNote> = HashMap::new();

    // User-global notes
    if user_dir.exists() {
        discover_in_dir(user_dir, NoteSource::UserGlobal, &mut notes)?;
    }

    // Repo-scoped notes (higher priority)
    if let Some(dir) = repo_dir {
        if dir.exists() {
            discover_in_dir(dir, NoteSource::RepoScoped, &mut notes)?;
        }
    }

    let mut result: Vec<DiscoveredNote> = notes.into_values().collect();
    result.sort_by(|a, b| a.topic.cmp(&b.topic));
    Ok(result)
}

/// Get a single note by topic name
pub fn get_note(
    topic: &str,
    user_dir: &Path,
    repo_dir: Option<&Path>,
) -> ForgeResult<DiscoveredNote> {
    // Check repo-scoped first (highest priority)
    if let Some(dir) = repo_dir {
        if let Some(note) = try_load_note(dir, topic, NoteSource::RepoScoped)? {
            return Ok(note);
        }
    }

    // Check user-global
    if let Some(note) = try_load_note(user_dir, topic, NoteSource::UserGlobal)? {
        return Ok(note);
    }

    Err(ForgeError::NotesTopicNotFound(topic.to_string()))
}

/// Resolve a notes directory path, expanding ~ to $HOME
pub fn resolve_notes_dir(dir: &str) -> PathBuf {
    if dir.starts_with('~') {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(dir.replacen('~', &home, 1));
        }
    }
    PathBuf::from(dir)
}

// --- Private helpers ---

fn discover_in_dir(
    dir: &Path,
    source: NoteSource,
    notes: &mut HashMap<String, DiscoveredNote>,
) -> ForgeResult<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| ForgeError::FileRead {
        path: dir.to_path_buf(),
        source: e,
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Some(note) = load_note_file(&path, source.clone())? {
                notes.insert(note.topic.clone(), note);
            }
        }
    }

    Ok(())
}

fn try_load_note(
    dir: &Path,
    topic: &str,
    source: NoteSource,
) -> ForgeResult<Option<DiscoveredNote>> {
    let path = dir.join(format!("{topic}.md"));
    if path.exists() {
        load_note_file(&path, source)
    } else {
        Ok(None)
    }
}

fn load_note_file(path: &Path, source: NoteSource) -> ForgeResult<Option<DiscoveredNote>> {
    let topic = match path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s.to_string(),
        None => return Ok(None),
    };

    let raw = std::fs::read_to_string(path).map_err(|e| ForgeError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;

    let (fm, body) = parse_frontmatter(&raw);

    Ok(Some(DiscoveredNote {
        topic,
        description: fm.description.unwrap_or_else(|| first_line(body)),
        category: fm.category.unwrap_or_else(|| "General".to_string()),
        content: body.to_string(),
        source,
    }))
}

struct Frontmatter {
    description: Option<String>,
    category: Option<String>,
}

/// Parse optional YAML-like frontmatter delimited by `---` lines.
/// Returns the parsed fields and the remaining body content.
fn parse_frontmatter(content: &str) -> (Frontmatter, &str) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (
            Frontmatter {
                description: None,
                category: None,
            },
            content,
        );
    }

    // Find the closing ---
    let after_open = &trimmed[3..].trim_start_matches(|c: char| c == '-');
    let after_open = after_open.trim_start_matches('\n');
    if let Some(close_pos) = after_open.find("\n---") {
        let fm_block = &after_open[..close_pos];
        let body_start = close_pos + 4; // skip "\n---"
        let body = after_open[body_start..].trim_start_matches('\n');

        let mut description = None;
        let mut category = None;

        for line in fm_block.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("description:") {
                description = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("category:") {
                category = Some(val.trim().to_string());
            }
        }

        (Frontmatter { description, category }, body)
    } else {
        // No closing ---, treat entire content as body
        (
            Frontmatter {
                description: None,
                category: None,
            },
            content,
        )
    }
}

/// Extract the first non-empty line as a fallback description
fn first_line(content: &str) -> String {
    content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("(no description)")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_with_fields() {
        let input = "---\ndescription: test desc\ncategory: Tools\n---\nBody here";
        let (fm, body) = parse_frontmatter(input);
        assert_eq!(fm.description.as_deref(), Some("test desc"));
        assert_eq!(fm.category.as_deref(), Some("Tools"));
        assert_eq!(body, "Body here");
    }

    #[test]
    fn test_parse_frontmatter_without() {
        let input = "Just some content\nMore lines";
        let (fm, body) = parse_frontmatter(input);
        assert!(fm.description.is_none());
        assert!(fm.category.is_none());
        assert_eq!(body, input);
    }

    #[test]
    fn test_first_line() {
        assert_eq!(first_line("\n\n  hello world\nmore"), "hello world");
        assert_eq!(first_line(""), "(no description)");
    }

    #[test]
    fn test_resolve_notes_dir() {
        let path = resolve_notes_dir("/absolute/path");
        assert_eq!(path, PathBuf::from("/absolute/path"));
    }
}
