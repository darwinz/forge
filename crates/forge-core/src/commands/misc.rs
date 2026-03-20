use crate::exec::CommandRunner;
use crate::error::ForgeResult;

/// Get weather for a location via wttr.in
pub fn weather(runner: &dyn CommandRunner, location: Option<&str>) -> ForgeResult<String> {
    let url = match location {
        Some(loc) => format!("https://wttr.in/{}?format=3", loc),
        None => "https://wttr.in/?format=3".to_string(),
    };
    let output = runner.run("curl", &["-s", &url])?;
    Ok(output.stdout)
}

/// Look up a word definition via an API
pub fn define(runner: &dyn CommandRunner, word: &str) -> ForgeResult<String> {
    let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", word);
    let output = runner.run("curl", &["-s", &url])?;
    if runner.is_dry_run() {
        return Ok(String::new());
    }
    Ok(format_definition(&output.stdout))
}

/// Parse the dictionaryapi.dev JSON response into readable text.
/// Best-effort: if parsing fails, return the raw response.
fn format_definition(json: &str) -> String {
    // The API returns an array of entries. We do a simple line-by-line parse
    // rather than pulling in serde_json. Extract word, phonetic, and meanings.
    let json = json.trim();
    if json.is_empty() || json.starts_with("{\"title\"") {
        return "No definition found.".to_string();
    }

    // Simple extraction: find "word", "definition" fields
    let mut result = String::new();
    let mut in_definitions = false;
    let mut def_count = 0;

    for line in json.lines() {
        let trimmed = line.trim().trim_matches(',');
        if trimmed.contains("\"word\"") {
            if let Some(val) = extract_json_string(trimmed) {
                if result.is_empty() {
                    result.push_str(&format!("  {val}\n"));
                }
            }
        } else if trimmed.contains("\"phonetic\"") {
            if let Some(val) = extract_json_string(trimmed) {
                if !val.is_empty() {
                    result.push_str(&format!("  {val}\n"));
                }
            }
        } else if trimmed.contains("\"partOfSpeech\"") {
            if let Some(val) = extract_json_string(trimmed) {
                result.push_str(&format!("\n  ({val})\n"));
                in_definitions = true;
                def_count = 0;
            }
        } else if in_definitions && trimmed.contains("\"definition\"") {
            if def_count < 3 {
                if let Some(val) = extract_json_string(trimmed) {
                    def_count += 1;
                    result.push_str(&format!("    {def_count}. {val}\n"));
                }
            }
        } else if trimmed == "]" && in_definitions {
            in_definitions = false;
        }
    }

    if result.is_empty() {
        // Fallback: return raw (truncated)
        let truncated: String = json.chars().take(500).collect();
        return truncated;
    }

    result
}

fn extract_json_string(s: &str) -> Option<String> {
    // Simple: find the value after the colon between quotes
    let colon = s.find(':')?;
    let after = s[colon + 1..].trim().trim_matches(',');
    if after.starts_with('"') && after.len() > 1 {
        let inner = &after[1..];
        if let Some(end) = inner.find('"') {
            return Some(inner[..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_definition_not_found() {
        assert_eq!(format_definition(""), "No definition found.");
        assert_eq!(
            format_definition("{\"title\":\"No Definitions Found\"}"),
            "No definition found."
        );
    }

    #[test]
    fn test_extract_json_string() {
        assert_eq!(
            extract_json_string(r#""word": "hello""#),
            Some("hello".to_string())
        );
        assert_eq!(
            extract_json_string(r#""phonetic": "/həˈloʊ/""#),
            Some("/həˈloʊ/".to_string())
        );
        assert_eq!(extract_json_string(r#""word": 42"#), None);
    }
}
