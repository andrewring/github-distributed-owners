use crate::owners_set::OwnersSet;
use anyhow::anyhow;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(PartialEq, Debug, Default)]
pub struct OwnersFileConfig {
    pub all_files: OwnersSet,
    pub pattern_overrides: HashMap<String, OwnersSet>,
}

impl OwnersFileConfig {
    pub fn from_text<S: AsRef<str>>(text: S) -> anyhow::Result<OwnersFileConfig> {
        let text = text.as_ref();
        let mut config = OwnersFileConfig::default();
        let current_set = &mut config.all_files;

        for line in text.lines() {
            let line = clean_line(line);
            if line.is_empty() {
                continue;
            }
            let is_set_line = current_set.maybe_process_set(line)?;
            if is_set_line {
                continue;
            }
            if line.contains(char::is_whitespace) {
                return Err(anyhow!(
                    "Invalid user/group '{}', cannot contain whitespace",
                    line
                ));
            }
            current_set.owners.insert(line.to_string());
        }

        Ok(config)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<OwnersFileConfig> {
        let text = fs::read_to_string(path)?;
        Self::from_text(text)
    }
}

/// Remove extraneous info in the line, such as comments and surrounding whitespace.
fn clean_line(line: &str) -> &str {
    line.find('#').map(|i| &line[..i]).unwrap_or(line).trim()
}

#[cfg(test)]
mod tests {
    use crate::owners_file::OwnersFileConfig;
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

        let parsed = OwnersFileConfig::from_text(input)?;
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

        let parsed = OwnersFileConfig::from_text(input)?;
        assert_eq!(parsed, expected);
        Ok(())
    }
}
