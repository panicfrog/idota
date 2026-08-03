//! Locate a Dota 2 installation on disk.
//!
//! Dota 2 is installed by Steam under `steamapps/common/dota 2 beta`. Steam may
//! keep games in multiple library folders, which are listed in
//! `steamapps/libraryfolders.vdf`, so we look there in addition to the well-known
//! default install locations. An explicit path to the executable, game root, or
//! config directory can always be provided as an override.

use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;

/// Directory name Steam uses for the Dota 2 installation.
pub const GAME_DIR_NAME: &str = "dota 2 beta";
/// Name of the Game State Integration configuration file we manage.
pub const CFG_FILE_NAME: &str = "gamestate_integration_dota2_assistant.cfg";

/// Relative locations of the Dota 2 executable, newest layouts first.
const EXECUTABLE_CANDIDATES: &[&str] = &[
    "game/bin/win64/dota2.exe",
    "game/bin/linuxsteamrt64/dota2",
    "game/bin/linux64/dota2",
    "game/bin/osx64/dota2.app/Contents/MacOS/dota2",
    "game/bin/osx64/dota2",
    "dota/bin/win64/dota2.exe",
    "game/dota/macos/dota2",
];

/// Relative locations of the GSI configuration directory, newest first.
const CFG_DIR_CANDIDATES: &[&str] = &["game/dota/cfg", "dota/cfg"];

/// A located Dota 2 installation.
#[derive(Debug, Clone)]
pub struct Dota2Install {
    /// Game root, e.g. `<steam library>/steamapps/common/dota 2 beta`.
    pub game_root: PathBuf,
    /// Path to the Dota 2 executable, when a known layout was found.
    pub executable: Option<PathBuf>,
    /// Directory where `gamestate_integration_*.cfg` files live.
    pub cfg_dir: PathBuf,
}

/// Find the Dota 2 installation, either from an explicit path or by searching
/// the usual Steam install locations.
pub fn find_install(override_path: Option<&Path>) -> Result<Dota2Install> {
    if let Some(path) = override_path {
        return install_from_override(path);
    }

    let mut tried: Vec<PathBuf> = Vec::new();
    for root in steam_roots() {
        for candidate in candidate_game_dirs_for_steam_root(&root) {
            tried.push(candidate.clone());
            if let Some(install) = install_from_game_root(&candidate) {
                log::debug!("found Dota 2 at `{}`", candidate.display());
                return Ok(install);
            }
        }
    }

    let tried = tried
        .iter()
        .map(|p| format!("  - {}", p.display()))
        .collect::<Vec<_>>()
        .join("\n");
    Err(anyhow::anyhow!(
        "could not find a Dota 2 installation. Looked in:\n{tried}\n\
         Pass the path to the game with `--path` (executable or game root)."
    ))
}

/// Resolve an explicit user-provided path into a [`Dota2Install`].
fn install_from_override(path: &Path) -> Result<Dota2Install> {
    let game_root = if path.is_file() {
        find_dir_ancestor(path, GAME_DIR_NAME).ok_or_else(|| {
            anyhow::anyhow!(
                "`{}` does not live inside a `{GAME_DIR_NAME}` directory",
                path.display()
            )
        })?
    } else {
        game_root_from_dir(path)
    };

    install_from_game_root(&game_root).ok_or_else(|| {
        anyhow::anyhow!(
            "`{}` does not look like a Dota 2 installation (no `game/dota` or `dota` directory)",
            game_root.display()
        )
    })
}

/// Find the ancestor directory with the given name (case-insensitive).
fn find_dir_ancestor(path: &Path, dir_name: &str) -> Option<PathBuf> {
    path.ancestors()
        .find(|a| {
            a.file_name()
                .map(|n| n.eq_ignore_ascii_case(dir_name))
                .unwrap_or(false)
        })
        .map(Path::to_path_buf)
}

/// Given a directory that may be the game root, `game/dota`, or
/// `game/dota/cfg`, resolve the actual game root.
fn game_root_from_dir(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase());

    match name.as_deref() {
        Some("cfg") => {
            // `<root>/game/dota/cfg` or legacy `<root>/dota/cfg`
            let grandparent = path.parent().and_then(Path::parent);
            match grandparent {
                Some(g) if g.file_name().is_some_and(|n| n.eq_ignore_ascii_case("game")) => {
                    g.parent().map(Path::to_path_buf).unwrap_or_else(|| g.to_path_buf())
                }
                Some(g) => g.to_path_buf(),
                None => path.to_path_buf(),
            }
        }
        Some("dota") => {
            // `<root>/game/dota` or legacy `<root>/dota`
            let parent = path.parent();
            match parent {
                Some(g) if g.file_name().is_some_and(|n| n.eq_ignore_ascii_case("game")) => {
                    g.parent().map(Path::to_path_buf).unwrap_or_else(|| g.to_path_buf())
                }
                Some(g) => g.to_path_buf(),
                None => path.to_path_buf(),
            }
        }
        _ => path.to_path_buf(),
    }
}

/// Build a [`Dota2Install`] for a game root, validating that it is really Dota 2.
fn install_from_game_root(game_root: &Path) -> Option<Dota2Install> {
    if !game_root.is_dir() {
        return None;
    }

    let has_game_data = ["game/dota", "dota"]
        .iter()
        .any(|d| game_root.join(d).is_dir());
    if !has_game_data {
        return None;
    }

    let cfg_dir = CFG_DIR_CANDIDATES
        .iter()
        .map(|c| game_root.join(c))
        .find(|p| p.is_dir())
        .unwrap_or_else(|| game_root.join(CFG_DIR_CANDIDATES[0]));

    let executable = EXECUTABLE_CANDIDATES
        .iter()
        .map(|c| game_root.join(c))
        .find(|p| p.is_file());

    Some(Dota2Install {
        game_root: game_root.to_path_buf(),
        executable,
        cfg_dir,
    })
}

/// Steam install roots for the current platform, plus any override from
/// the `DOTA2_ASSISTANT_STEAM_ROOT` environment variable.
fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    match env::var("DOTA2_ASSISTANT_STEAM_ROOT") {
        Ok(value) if !value.is_empty() => roots.push(PathBuf::from(value)),
        _ => {}
    }

    #[cfg(windows)]
    {
        if let Some(pf) = env::var_os("ProgramFiles(x86)") {
            roots.push(PathBuf::from(pf).join("Steam"));
        }
        if let Some(pf) = env::var_os("ProgramFiles") {
            roots.push(PathBuf::from(pf).join("Steam"));
        }
        if let Some(home) = env::var_os("USERPROFILE") {
            roots.push(PathBuf::from(home).join("Steam"));
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Library/Application Support/Steam"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(home) = env::var_os("HOME") {
            roots.push(PathBuf::from(home).join(".steam/steam"));
            roots.push(PathBuf::from(home).join(".local/share/Steam"));
            roots.push(PathBuf::from(home).join("Steam"));
            roots.push(PathBuf::from(
                home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
            ));
        }
        roots.push(PathBuf::from("/usr/share/steam"));
    }

    roots
}

/// All game directories that may contain Dota 2 for a given Steam root,
/// including every library folder declared in `libraryfolders.vdf`.
fn candidate_game_dirs_for_steam_root(steam_root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let steamapps = steam_root.join("steamapps");

    // The Steam install directory itself is always a library folder.
    candidates.push(steamapps.join("common").join(GAME_DIR_NAME));

    let vdf_path = steamapps.join("libraryfolders.vdf");
    if let Ok(text) = std::fs::read_to_string(&vdf_path) {
        for library in library_paths_from_vdf(&text) {
            // Modern Steam stores the full `<library>/steamapps` path; older
            // versions stored just the library root. Cover both.
            candidates.push(library.join("common").join(GAME_DIR_NAME));
            candidates.push(library.join("steamapps").join("common").join(GAME_DIR_NAME));
        }
    }

    candidates
}

/// Extract the values of every `"path"` key from a `libraryfolders.vdf` file.
pub fn library_paths_from_vdf(text: &str) -> Vec<PathBuf> {
    // Splitting on `"` yields the quoted string contents at odd indices.
    let tokens: Vec<String> = text
        .split('"')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, part)| unescape_vdf(part))
        .collect();

    tokens
        .windows(2)
        .filter(|pair| pair[0].eq_ignore_ascii_case("path"))
        .map(|pair| PathBuf::from(&pair[1]))
        .collect()
}

/// Decode VDF string escapes (`\\`, `\"`, `\n`, `\t`).
fn unescape_vdf(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dota2-assistant-discovery-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn parses_modern_libraryfolders_vdf() {
        let vdf = r#"
"libraryfolders"
{
    "contentstatsid" "-3072164613065168484"
    "0"
    {
        "path" "C:\\Program Files (x86)\\Steam\\steamapps"
        "label" ""
        "apps"
        {
            "570" "1896152561"
        }
    }
    "1"
    {
        "path" "D:\\Games\\SteamLibrary\\steamapps"
    }
}
"#;
        let paths = library_paths_from_vdf(vdf);
        assert_eq!(
            paths,
            vec![
                PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps"),
                PathBuf::from(r"D:\Games\SteamLibrary\steamapps"),
            ]
        );
    }

    #[test]
    fn parses_legacy_libraryfolders_vdf() {
        let vdf = r#"
"libraryfolders"
{
    "0"
    {
        "path" "D:\\SteamLibrary"
    }
}
"#;
        assert_eq!(
            library_paths_from_vdf(vdf),
            vec![PathBuf::from(r"D:\SteamLibrary")]
        );
    }

    #[test]
    fn finds_install_in_steam_root() {
        let base = temp_dir("steam-root");
        let steam = base.join("Steam");
        let game = steam.join("steamapps").join("common").join(GAME_DIR_NAME);
        fs::create_dir_all(game.join("game/dota/cfg")).unwrap();
        fs::create_dir_all(game.join("game/bin/win64")).unwrap();
        fs::write(game.join("game/bin/win64/dota2.exe"), "").unwrap();

        let found = find_install(Some(&game)).expect("explicit game root should work");
        assert_eq!(found.game_root, game);
        assert_eq!(found.cfg_dir, game.join("game/dota/cfg"));
        assert_eq!(
            found.executable,
            Some(game.join("game/bin/win64/dota2.exe"))
        );

        let found = find_install(Some(&game.join("game/bin/win64/dota2.exe")))
            .expect("explicit executable should work");
        assert_eq!(found.game_root, game);

        // Search through the steam root + libraryfolders.vdf
        let steamapps_dir = steam.join("steamapps");
        fs::create_dir_all(&steamapps_dir).unwrap();
        let vdf_path = steamapps_dir.join("libraryfolders.vdf");
        fs::write(
            &vdf_path,
            format!("\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} }}", steam.display()),
        )
        .unwrap();
        unsafe {
            std::env::set_var("DOTA2_ASSISTANT_STEAM_ROOT", &steam);
        }
        let found = find_install(None).expect("auto search should find the game");
        assert_eq!(found.game_root, game);
        unsafe {
            std::env::remove_var("DOTA2_ASSISTANT_STEAM_ROOT");
        }

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn rejects_non_dota_directory() {
        let base = temp_dir("not-dota");
        fs::create_dir_all(base.join("somewhere")).unwrap();
        let result = find_install(Some(&base));
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&base);
    }
}
