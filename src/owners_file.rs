use crate::owners_set::OwnersSet;
use anyhow::anyhow;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(PartialEq, Debug, Default)]
pub struct OwnersFileConfig {
    pub all_files: OwnersSet,
    pub pattern_overrides: HashMap<String, OwnersSet>,
}

impl OwnersFileConfig {
    pub fn from_file<P0: AsRef<Path>, P1: AsRef<Path>>(
        path: P0,
        repo_base: P1,
    ) -> anyhow::Result<OwnersFileConfig> {
        let path_ref = path.as_ref();
        let text = fs::read_to_string(path_ref)?;
        Self::from_text(&text, path.as_ref(), repo_base.as_ref())
    }

    fn from_text<S: AsRef<str>, P0: AsRef<Path>, P1: AsRef<Path>>(
        text: S,
        path: P0,
        repo_base: P1,
    ) -> anyhow::Result<OwnersFileConfig> {
        let mut config = OwnersFileConfig::default();
        Self::parse_text(
            &mut config,
            text.as_ref(),
            path.as_ref(),
            repo_base.as_ref(),
            (&mut HashMap::new()).into(),
        )?;
        Ok(config)
    }

    fn parse_text<P0: AsRef<Path>, P1: AsRef<Path>>(
        config: &mut OwnersFileConfig,
        text: &str,
        path: P0,
        repo_base: P1,
        seen_owners_files: &mut HashMap<PathBuf, Option<PathBuf>>,
    ) -> anyhow::Result<()> {
        // `active_pattern_key` tracks the current context.
        // `None`: Modifying `config.all_files`.
        // `Some(key)`: Modifying `config.pattern_overrides` for the given key.
        let mut active_pattern_key: Option<String> = None;
        let source = path
            .as_ref()
            .to_str()
            .expect("Error converting file path to string");

        if seen_owners_files.is_empty() {
            seen_owners_files.insert(path.as_ref().to_path_buf(), None);
        }

        for (i, raw_line) in text.lines().enumerate() {
            let line = clean_line(raw_line);
            if line.is_empty() {
                continue;
            }
            let line_number = i + 1;

            if let Some(include_file) = maybe_get_include(line)
                .map_err(|error| anyhow!("{} Found at {}:{}", error, source, line_number))?
            {
                if active_pattern_key.is_some() {
                    return Err(anyhow!(
                        "include is not allowed in path-specific sections. Found at {}:{}",
                        source,
                        line_number
                    ));
                }

                let include_path =
                    resolve_include_path(repo_base.as_ref(), path.as_ref(), &include_file)
                        .map_err(|error| {
                            anyhow!("{} Found at {}:{}", error, source, line_number)
                        })?;

                let include_text = fs::read_to_string(&include_path).map_err(|error| {
                    anyhow!(
                        "{} Found at {}:{}",
                        error,
                        include_path.display(),
                        line_number
                    )
                })?;

                check_no_circular_include(&include_path, &seen_owners_files)?;
                seen_owners_files.insert(include_path.clone(), Some(path.as_ref().to_path_buf()));

                Self::parse_text(
                    config,
                    &include_text,
                    &include_path,
                    repo_base.as_ref(),
                    seen_owners_files,
                )?;
                continue;
            }

            // We scope this borrow to ensure we don't hold onto the mutable reference longer than necessary,
            // since it can cause issues with recursively borrowing the `config` object above.
            let current_set: &mut OwnersSet = {
                if let Some(ref key) = active_pattern_key {
                    config.pattern_overrides.entry(key.clone()).or_default()
                } else {
                    &mut config.all_files
                }
            };

            let is_set_line = current_set
                .maybe_process_set(line)
                .map_err(|error| anyhow!("{} Encountered at {}:{}", error, source, line_number))?;
            if is_set_line {
                // If there's more than one seen_owners_files, then we're inside an include where
                // set statements aren't allowed.
                if seen_owners_files.len() > 1 {
                    return Err(anyhow!(
                        "set statements are not allowed inside includes. Found at {}:{}",
                        source,
                        line_number
                    ));
                }
                continue;
            }

            if let Some(new_file_pattern) = maybe_get_file_pattern(line) {
                active_pattern_key = Some(new_file_pattern);
                continue;
            }

            if line.contains(char::is_whitespace) {
                return Err(anyhow!(
                    "Invalid user/group '{}' cannot contain whitespace. Found at {}:{}",
                    line,
                    source,
                    line_number
                ));
            }
            current_set.owners.insert(line.to_string());
        }
        seen_owners_files.remove(path.as_ref());
        Ok(())
    }
}

/// Remove extraneous info in the line, such as comments and surrounding whitespace.
fn clean_line(line: &str) -> &str {
    line.find('#').map(|i| &line[..i]).unwrap_or(line).trim()
}

/// Parses a file pattern line, e.g., `[*.rs]`.
fn maybe_get_file_pattern(line: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\s*\[\s*(?<pattern>\S+)\s*]\s*$").unwrap();
    }
    if let Some(captures) = RE.captures(line) {
        let pattern = &captures["pattern"];
        Some(pattern.to_string())
    } else {
        None
    }
}

/// Parses an include directive, e.g., `include path/to/another/OWNERS`.
fn maybe_get_include(line: &str) -> anyhow::Result<Option<String>> {
    lazy_static! {
        // Ensures the path is non-empty and doesn't contain whitespace.
        static ref RE: Regex = Regex::new(r"^\s*include\s+(?<path>\S+)\s*$").unwrap();
        static ref MALFORMED_RE: Regex = Regex::new(r"^\s*include\s*$").unwrap();
    }
    if let Some(captures) = RE.captures(line) {
        let path = captures["path"].to_string();
        dbg!(captures["path"].to_string());
        if path.is_empty() {
            return Err(anyhow!("Invalid include. Expected non-empty include path."));
        }

        Ok(Some(path))
    } else if MALFORMED_RE.is_match(line) {
        Err(anyhow!(
            "Invalid include format '{}'. Expected 'include <path>'.",
            line,
        ))
    } else if line.to_lowercase().starts_with("include ") {
        Err(anyhow!(
            "Invalid include format '{}'. Expected 'include <path>'.",
            line,
        ))
    } else {
        Ok(None)
    }
}

fn resolve_include_path<P0: AsRef<Path>, P1: AsRef<Path>, P2: AsRef<Path>>(
    repo_base: P0,
    current_path: P1,
    include_path: P2,
) -> anyhow::Result<PathBuf> {
    let repo_base_path = repo_base.as_ref();
    let current_path_ref = current_path.as_ref();
    let include_path_ref = include_path.as_ref();

    let current_dir = current_path_ref.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "current_path has no parent directory: {:?}",
            current_path_ref
        )
    })?;

    let path = if include_path_ref.is_absolute() {
        repo_base_path.join(
            include_path_ref
                .strip_prefix("/")
                .or_else(|_| include_path_ref.strip_prefix("\\"))
                .unwrap_or(include_path_ref),
        )
    } else {
        current_dir.join(include_path_ref)
    };

    let canonicalized_path = fs::canonicalize(&path).map_err(|error| {
        anyhow!(
            "Failed to canonicalize include path '{}': {}",
            path.display(),
            error
        )
    })?;
    if !canonicalized_path.starts_with(repo_base_path) {
        return Err(anyhow!(
            "Include path '{}' is outside the repository base '{}'.",
            canonicalized_path.display(),
            repo_base_path.display()
        ));
    }
    Ok(canonicalized_path)
}

fn check_no_circular_include(
    path: &PathBuf,
    seen_owners_files: &HashMap<PathBuf, Option<PathBuf>>,
) -> anyhow::Result<()> {
    if !seen_owners_files.contains_key(path) {
        return Ok(());
    }

    let mut chain = vec![path.clone()];
    let mut current = path;

    // Walk the reverse linked list to get a nice printable chain of includes.
    while let Some(Some(parent)) = seen_owners_files.get(current) {
        chain.push(parent.clone());
        current = parent;
    }

    // The error message is easier to read if it's in forward order rather than reverse.
    chain.reverse();

    let message = chain
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n     -> ");

    Err(anyhow!("Cycle detected in includes: \n.   {}", message))
}

#[cfg(test)]
mod tests {
    use crate::owners_file::{maybe_get_file_pattern, maybe_get_include, OwnersFileConfig};
    use crate::owners_set::OwnersSet;
    use indoc::indoc;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn parse_blanket_owners_only() -> anyhow::Result<()> {
        let input = indoc! {"\
            ada.lovelace
            grace.hopper
            margaret.hamilton
            "
        };
        let expected = OwnersFileConfig {
            all_files: OwnersSet {
                inherit: None,
                owners: vec!["ada.lovelace", "grace.hopper", "margaret.hamilton"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            },
            pattern_overrides: HashMap::default(),
        };

        let parsed = OwnersFileConfig::from_text(input, "test data", "test data")?;
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn parse_blanket_owners_with_inherit() -> anyhow::Result<()> {
        let input = indoc! {"\
            set inherit = false
            ada.lovelace
            grace.hopper
            margaret.hamilton
            "
        };
        let expected = OwnersFileConfig {
            all_files: OwnersSet {
                inherit: Some(false),
                owners: vec!["ada.lovelace", "grace.hopper", "margaret.hamilton"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            },
            pattern_overrides: HashMap::default(),
        };

        let parsed = OwnersFileConfig::from_text(input, "test data", "test data")?;
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn parse_blanket_with_pattern_overrides() -> anyhow::Result<()> {
        let input = indoc! {"\
            ada.lovelace
            grace.hopper
            margaret.hamilton

            [*.rs]
            katherine.johnson
            "
        };
        let expected = OwnersFileConfig {
            all_files: OwnersSet {
                inherit: None,
                owners: vec!["ada.lovelace", "grace.hopper", "margaret.hamilton"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            },
            pattern_overrides: HashMap::from([(
                "*.rs".to_string(),
                OwnersSet {
                    inherit: None,
                    owners: vec!["katherine.johnson"]
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<String>>(),
                },
            )]),
        };

        let parsed = OwnersFileConfig::from_text(input, "test data", "test data")?;
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn test_maybe_get_file_pattern() {
        assert_eq!(maybe_get_file_pattern("[*.rs]"), Some("*.rs".to_string()));
        assert_eq!(maybe_get_file_pattern("[foo.*]"), Some("foo.*".to_string()));
        assert_eq!(
            maybe_get_file_pattern("  [  bar.*  ]  "),
            Some("bar.*".to_string())
        );
        assert_eq!(maybe_get_file_pattern("ada.lovelace"), None);
        assert_eq!(maybe_get_file_pattern(""), None);
        assert_eq!(maybe_get_file_pattern("set inherit = false"), None);
    }

    #[test]
    fn test_maybe_get_include() -> anyhow::Result<()> {
        assert_eq!(
            maybe_get_include("include foo/bar.owners")?,
            Some("foo/bar.owners".to_string())
        );
        assert_eq!(
            maybe_get_include("  include   my_path   ")?,
            Some("my_path".to_string())
        );
        assert!(maybe_get_include("include").is_err());
        assert!(maybe_get_include("include ").is_err());
        assert!(maybe_get_include("include path with spaces").is_err()); // Regex `\S+` handles this.
        assert_eq!(maybe_get_include("not an include")?, None);
        Ok(())
    }
}
