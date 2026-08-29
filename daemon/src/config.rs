use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ha::registry::AreaPrefs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub base_url: String,
    pub areas: AreaPrefs,
    pub favorites: Vec<String>,
    pub pinned_tabs: Vec<String>,
    pub allow_sensitive_ipc: bool,
    pub show_favorites: bool,
    pub imported_dashboard_prefs: bool,
    pub display_name_overrides: std::collections::BTreeMap<String, String>,
    pub selected_tab: String,
}

impl Config {
    pub fn path() -> PathBuf {
        if let Ok(explicit) = std::env::var("ATRIUM_CONFIG") {
            return PathBuf::from(explicit);
        }
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".config")
            });
        base.join("atrium").join("config.json")
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Ok(Self::default());
        };
        match serde_json::from_str(&raw) {
            Ok(config) => Ok(config),
            // Starting from defaults would be silent data loss: the next save
            // overwrites the file, and one room change triggers a save.
            Err(e) => {
                let kept = path.with_extension("json.corrupt");
                let note = match std::fs::rename(path, &kept) {
                    Ok(()) => format!("kept a copy at {}", kept.display()),
                    Err(rename) => format!("could not set it aside: {rename}"),
                };
                Err(format!(
                    "{} could not be read ({e}); {note}",
                    path.display()
                ))
            }
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
        let json = serde_json::to_string_pretty(self)?;
        let temporary = path.with_extension("json.tmp");
        {
            use std::io::Write;
            // The file names the instance address, every room and every pinned
            // device; the ambient umask would leave that world-readable.
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temporary)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
        }
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
        std::fs::rename(&temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("atrium-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_start_disconnected_with_areas_visible() {
        let config = Config::default();
        assert!(config.base_url.is_empty());
        assert!(config.favorites.is_empty());
        assert!(!config.show_favorites);
        assert!(config.areas.hide_empty_areas);
    }

    #[test]
    fn a_round_trip_preserves_area_preferences() {
        let path = temp_dir().join("round-trip.json");
        let config = Config {
            base_url: "https://ha.example.com".into(),
            areas: AreaPrefs {
                order: vec!["office".into(), "garage".into()],
                hidden: vec!["patio".into()],
                ..Default::default()
            },
            imported_dashboard_prefs: true,
            ..Default::default()
        };
        config.save(&path).unwrap();

        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.base_url, "https://ha.example.com");
        assert_eq!(loaded.areas.order, ["office", "garage"]);
        assert_eq!(loaded.areas.hidden, ["patio"]);
        assert!(loaded.imported_dashboard_prefs);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn sensitive_ipc_is_off_until_asked_for() {
        assert!(!Config::default().allow_sensitive_ipc);
    }

    #[test]
    fn pinned_tabs_round_trip() {
        let path = temp_dir().join("pinned.json");
        let config = Config {
            pinned_tabs: vec!["area:office".into(), "unassigned".into()],
            ..Default::default()
        };
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap().pinned_tabs, ["area:office", "unassigned"]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn the_config_file_is_camel_case_throughout() {
        let path = temp_dir().join("camel.json");
        let config = Config {
            areas: AreaPrefs { hide_empty_areas: true, ..Default::default() },
            ..Default::default()
        };
        config.save(&path).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("hideEmptyAreas"));
        assert!(written.contains("hideEntitiesWithoutArea"));
        assert!(!written.contains("hide_empty_areas"));
        assert!(Config::load(&path).unwrap().areas.hide_empty_areas);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_token_is_never_written_even_if_one_is_smuggled_in() {
        let path = temp_dir().join("no-token.json");
        let config = Config {
            base_url: "https://ha.example.com".into(),
            ..Default::default()
        };
        config.save(&path).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        for banned in ["token", "access_token", "password", "secret"] {
            assert!(
                !written.to_lowercase().contains(banned),
                "{banned} appeared in the config file"
            );
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_corrupt_file_is_reported_and_kept_rather_than_overwritten() {
        let path = temp_dir().join("corrupt.json");
        let kept = path.with_extension("json.corrupt");
        std::fs::remove_file(&kept).ok();
        std::fs::write(&path, b"{ this is not json").unwrap();

        let outcome = Config::load(&path);
        assert!(outcome.is_err(), "a corrupt config must not read as defaults");
        assert!(kept.exists(), "the unreadable file must be kept");
        assert_eq!(std::fs::read(&kept).unwrap(), b"{ this is not json");
        std::fs::remove_file(&kept).ok();
    }

    #[test]
    fn the_config_file_is_not_world_readable() {
        let path = temp_dir().join("modes.json");
        Config::default().save(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config was written as {mode:o}");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        let loaded = Config::load(Path::new("/nonexistent/atrium/config.json")).unwrap();
        assert!(loaded.base_url.is_empty());
    }

    #[test]
    fn unknown_and_missing_fields_survive_a_version_skew() {
        let path = temp_dir().join("partial.json");
        std::fs::write(
            &path,
            br#"{"baseUrl":"https://ha.example.com","somethingNew":42}"#,
        )
        .unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.base_url, "https://ha.example.com");
        assert!(loaded.areas.hide_empty_areas, "missing fields take their default");
        std::fs::remove_file(&path).ok();
    }
}
