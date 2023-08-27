use crate::owners_set::OwnersSet;
use anyhow::anyhow;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(PartialEq, Debug, Default)]
pub struct OwnersFileConfig {
    pub all_files: OwnersSet,
    pub pattern_overrides: HashMap<String, OwnersSet>,
}

impl OwnersFileConfig {
    pub fn from_text<S0: AsRef<str>, S1: AsRef<str>>(
        text: S0,
        source: S1,
    ) -> anyhow::Result<OwnersFileConfig> {
        let text = text.as_ref();
        let mut config = OwnersFileConfig::default();
        let mut current_set = &mut config.all_files;

        for (line_number, line) in text.lines().enumerate() {
            let line = clean_line(line);
            if line.is_empty() {
                continue;
            }
            let is_set_line = current_set.maybe_process_set(line).map_err(|error| {
                anyhow!(
                    "{} Encountered at {}:{}",
                    error.to_string(),
                    source.as_ref(),
                    line_number
                )
            })?;
            if is_set_line {
                continue;
            }
            if let Some(new_file_pattern) = maybe_get_file_pattern(line) {
                config
                    .pattern_overrides
                    .insert(new_file_pattern.clone(), OwnersSet::default());
                current_set = config
                    .pattern_overrides
                    .get_mut(new_file_pattern.as_str())
                    .unwrap();
                continue;
            }
            if line.contains(char::is_whitespace) {
                return Err(anyhow!(
                    "Invalid user/group '{}' cannot contain whitespace. Found at {}:{}",
                    line,
                    source.as_ref(),
                    line_number
                ));
            }
            current_set.owners.insert(line.to_string());
        }

        Ok(config)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<OwnersFileConfig> {
        let text = fs::read_to_string(&path)?;
        Self::from_text(
            text,
            path.as_ref()
                .to_str()
                .expect("Error converting file path to string"),
        )
    }
}

/// Remove extraneous info in the line, such as comments and surrounding whitespace.
fn clean_line(line: &str) -> &str {
    line.find('#').map(|i| &line[..i]).unwrap_or(line).trim()
}

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

#[cfg(test)]
mod tests {
    use crate::owners_file::{maybe_get_file_pattern, OwnersFileConfig};
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

        let parsed = OwnersFileConfig::from_text(input, "test data")?;
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

        let parsed = OwnersFileConfig::from_text(input, "test data")?;
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

        let parsed = OwnersFileConfig::from_text(input, "test data")?;
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
}
