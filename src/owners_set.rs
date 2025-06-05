use anyhow::anyhow;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;

#[derive(PartialEq, Debug, Default, Eq)]
pub struct OwnersSet {
    pub inherit: Option<bool>,
    pub owners: HashSet<String>,
}

impl OwnersSet {
    /// Evaluates the line for set variable syntax. If found, the variable specified will be updated
    /// to match the value specified.
    ///
    /// returns whether the line was a set line
    pub fn maybe_process_set(&mut self, line: &str) -> anyhow::Result<bool> {
        if !line.starts_with("set ") {
            return Ok(false);
        }
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"^\s*set\s(?<variable>\w+)\s*=\s*(?<value>\w+)\s*$").unwrap();
        }
        if let Some(captures) = RE.captures(line) {
            let variable = &captures["variable"];
            let value = &captures["value"];
            match variable {
                "inherit" => match value {
                    "true" => {
                        self.inherit = Some(true);
                    }
                    "false" => {
                        self.inherit = Some(false);
                    }
                    _ => {
                        return Err(anyhow!(
                            "Invalid value for inherit '{}': Must be 'true' or 'false'.",
                            value
                        ))
                    }
                },
                _ => {
                    return Err(anyhow!("Invalid set variable '{}'", variable,));
                }
            }
        } else {
            return Err(anyhow!(
                "Invalid set format '{}']. Expected 'set <variable> = <value>'.",
                line,
            ));
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::owners_set::OwnersSet;

    #[test]
    fn process_set_non_set() -> anyhow::Result<()> {
        let mut owners_set = OwnersSet::default();
        assert!(!owners_set.maybe_process_set("ada.lovelace")?);
        Ok(())
    }

    #[test]
    fn process_set_nominal_true() -> anyhow::Result<()> {
        let mut owners_set = OwnersSet::default();
        assert!(owners_set.maybe_process_set("set inherit = true")?);
        assert_eq!(owners_set.inherit, Some(true));
        Ok(())
    }

    #[test]
    fn process_set_nominal_false() -> anyhow::Result<()> {
        let mut owners_set = OwnersSet::default();
        assert!(owners_set.maybe_process_set("set inherit = false")?);
        assert_eq!(owners_set.inherit, Some(false));
        Ok(())
    }

    #[test]
    fn process_set_invalid() -> anyhow::Result<()> {
        let mut owners_set = OwnersSet::default();
        assert!(is_error_with_text(
            owners_set.maybe_process_set("set inherit = not_a_bool"),
            "Invalid value"
        ));
        assert!(is_error_with_text(
            owners_set.maybe_process_set("set foo = bar"),
            "Invalid set variable"
        ));
        Ok(())
    }

    fn is_error_with_text<T>(result: anyhow::Result<T>, contents: &str) -> bool {
        if result.is_ok() {
            return false;
        }
        let error = result.err().unwrap();
        let message = error.to_string();
        if message.contains(contents) {
            return true;
        }
        eprintln!("Error message missing expected '{contents}', in \n    '{message}'");
        false
    }
}
