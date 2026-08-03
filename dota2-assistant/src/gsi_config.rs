//! Generate and install the Dota 2 Game State Integration (GSI) configuration
//! file into the game's `cfg` directory.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::discovery::CFG_FILE_NAME;

/// The GSI data sections we enable in the configuration file.
const DATA_SECTIONS: &[&str] = &[
    "buildings",
    "provider",
    "map",
    "player",
    "hero",
    "abilities",
    "items",
    "draft",
    "wearables",
];

/// Build the contents of the `gamestate_integration_*.cfg` file.
///
/// `uri` must match the URI the assistant server is bound to, e.g.
/// `http://127.0.0.1:53000/`. `token` is optional; when present Dota 2
/// includes it as `auth.token` in every payload.
pub fn config_contents(uri: &str, token: Option<&str>) -> String {
    let mut out = String::from("\"dota2-assistant Configuration\"\n{\n");

    out.push_str(&format!("    \"uri\"               \"{uri}\"\n"));
    out.push_str("    \"timeout\"           \"5.0\"\n");
    out.push_str("    \"buffer\"            \"0.1\"\n");
    out.push_str("    \"throttle\"          \"0.1\"\n");
    out.push_str("    \"heartbeat\"         \"30.0\"\n");

    out.push_str("    \"data\"\n    {\n");
    for section in DATA_SECTIONS {
        out.push_str(&format!("        \"{section}\"         \"1\"\n"));
    }
    out.push_str("    }\n");

    if let Some(token) = token {
        out.push_str("    \"auth\"\n    {\n");
        out.push_str(&format!(
            "        \"token\"         \"{}\"\n",
            escape_vdf(token)
        ));
        out.push_str("    }\n");
    }

    out.push_str("}\n");
    out
}

/// Escape a value for embedding inside a quoted VDF string: backslashes and
/// double quotes must be escaped so Dota 2 can still parse the config file.
fn escape_vdf(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out
}

/// Write the GSI configuration file into `cfg_dir`, creating the directory
/// if needed. Returns the path of the written file.
pub fn write_config(cfg_dir: &Path, uri: &str, token: Option<&str>) -> Result<PathBuf> {
    fs::create_dir_all(cfg_dir).with_context(|| {
        format!(
            "failed to create GSI configuration directory `{}`",
            cfg_dir.display()
        )
    })?;

    let path = cfg_dir.join(CFG_FILE_NAME);
    fs::write(&path, config_contents(uri, token)).with_context(|| {
        format!("failed to write GSI configuration file `{}`", path.display())
    })?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const TEST_URI: &str = "http://127.0.0.1:53000/";

    #[test]
    fn contents_include_uri_and_sections() {
        let contents = config_contents(TEST_URI, None);
        assert!(contents.starts_with("\"dota2-assistant Configuration\""));
        assert!(contents.contains(&format!("\"uri\"               \"{TEST_URI}\"")));
        for section in DATA_SECTIONS {
            assert!(contents.contains(&format!("\"{section}\"")));
        }
        assert!(!contents.contains("\"auth\""));
    }

    #[test]
    fn contents_include_auth_when_token_given() {
        let contents = config_contents(TEST_URI, Some("secret123"));
        assert!(contents.contains("\"token\"         \"secret123\""));
    }

    #[test]
    fn token_is_escaped_for_vdf() {
        let contents = config_contents(TEST_URI, Some("a\"b\\c"));
        assert!(contents.contains(r#""token"         "a\"b\\c""#));
    }

    #[test]
    fn write_config_creates_directory_and_file() {
        let dir = std::env::temp_dir().join(format!(
            "dota2-assistant-gsi-config-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let nested = dir.join("game/dota/cfg");

        let path = write_config(&nested, TEST_URI, Some("abc")).unwrap();
        assert_eq!(path.file_name().unwrap(), CFG_FILE_NAME);
        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(written, config_contents(TEST_URI, Some("abc")));

        let _ = fs::remove_dir_all(&dir);
    }
}
