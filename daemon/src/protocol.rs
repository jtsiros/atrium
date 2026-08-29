use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Command {
    Connect,
    Disconnect,
    SetUrl { url: String },
    SetToken {
        token: String,
        #[serde(default)]
        url: Option<String>,
    },
    ForgetToken,
    Action {
        entity_id: String,
        action: String,
        #[serde(default)]
        data: Value,
    },
    SetAreaPrefs {
        // Absent means "leave it alone". An omitted list must never be read as
        // an instruction to clear a hand-built room order.
        #[serde(default)]
        order: Option<Vec<String>>,
        #[serde(default)]
        hidden: Option<Vec<String>>,
        #[serde(default)]
        hide_empty_areas: Option<bool>,
        #[serde(default)]
        hide_entities_without_area: Option<bool>,
    },
    ImportDashboardPrefs,
    SetFavorites {
        #[serde(default)]
        ids: Vec<String>,
        #[serde(default)]
        show: Option<bool>,
    },
    SetSelectedTab { tab: String },
    SetPinnedTabs { ids: Vec<String> },
    Refresh,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    Unconfigured,
    NeedsToken,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
    Offline,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "ev", rename_all = "camelCase")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    Status {
        state: ConnectionState,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ha_version: Option<String>,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        plaintext: bool,
    },
    Tabs { tabs: Vec<crate::ha::registry::Tab> },
    Areas { areas: Vec<crate::ha::registry::AreaChoice> },
    Rows { rows: Vec<crate::ha::model::Row> },
    Row { row: crate::ha::model::Row },
    Dropped { entity_id: String },
    #[serde(rename_all = "camelCase")]
    Config {
        favorites: Vec<String>,
        pinned_tabs: Vec<String>,
        allow_sensitive_ipc: bool,
        show_favorites: bool,
        selected_tab: String,
        imported_dashboard_prefs: bool,
        base_url: String,
        hide_empty_areas: bool,
        hide_entities_without_area: bool,
    },
    #[serde(rename_all = "camelCase")]
    Log {
        level: Level,
        /// Unix milliseconds, so the panel can show "4m ago" without a clock
        /// of its own.
        at: u64,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        entity_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Envelope {
    pub v: u32,
    pub gen: u64,
    #[serde(flatten)]
    pub event: Event,
}

impl Envelope {
    pub fn new(generation: u64, event: Event) -> Self {
        Self { v: VERSION, gen: generation, event }
    }

    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            format!(r#"{{"v":{VERSION},"gen":0,"ev":"error","message":"could not encode event: {e}"}}"#)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> Command {
        serde_json::from_str(line).unwrap_or_else(|e| panic!("{line} should parse: {e}"))
    }

    #[test]
    fn commands_parse_from_their_wire_form() {
        assert!(matches!(parse(r#"{"cmd":"connect"}"#), Command::Connect));
        assert!(matches!(parse(r#"{"cmd":"refresh"}"#), Command::Refresh));

        let Command::Action { entity_id, action, .. } = parse(r#"{"cmd":"action","entityId":"light.desk","action":"toggle"}"#)
        else {
            panic!("expected an action");
        };
        assert_eq!(entity_id, "light.desk");
        assert_eq!(action, "toggle");
    }

    #[test]
    fn a_token_carries_the_address_it_was_entered_against() {
        let Command::SetToken { token, url } =
            parse(r#"{"cmd":"setToken","token":"abc","url":"https://ha.example.com"}"#)
        else {
            panic!("expected a token");
        };
        assert_eq!(token, "abc");
        assert_eq!(url.as_deref(), Some("https://ha.example.com"));

        let Command::SetToken { url, .. } = parse(r#"{"cmd":"setToken","token":"abc"}"#) else {
            panic!("expected a token");
        };
        assert_eq!(url, None);
    }

    #[test]
    fn area_preferences_accept_partial_updates() {
        let Command::SetAreaPrefs { order, hidden, hide_empty_areas, hide_entities_without_area } =
            parse(r#"{"cmd":"setAreaPrefs","hidden":["patio"]}"#)
        else {
            panic!("expected area prefs");
        };
        assert_eq!(order, None);
        assert_eq!(hidden.as_deref(), Some(&["patio".to_string()][..]));
        assert_eq!(hide_empty_areas, None);
        assert_eq!(hide_entities_without_area, None);
    }

    #[test]
    fn an_unknown_command_is_an_error_not_a_panic() {
        assert!(serde_json::from_str::<Command>(r#"{"cmd":"launchMissiles"}"#).is_err());
        assert!(serde_json::from_str::<Command>("not json").is_err());
    }

    #[test]
    fn every_event_carries_the_version_and_generation() {
        let line = Envelope::new(7, Event::Log {
            level: Level::Error,
            at: 1,
            text: "nope".into(),
            entity_id: None,
        })
        .to_line();
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["v"], VERSION);
        assert_eq!(value["gen"], 7);
        assert_eq!(value["ev"], "log");
        assert_eq!(value["text"], "nope");
    }

    #[test]
    fn a_log_entry_carries_a_level_and_a_timestamp() {
        let line = Envelope::new(1, Event::Log {
            level: Level::Warn,
            at: 1_788_000_000_000,
            text: "Home Assistant refused light.turn_on".into(),
            entity_id: Some("light.desk".into()),
        })
        .to_line();
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["ev"], "log");
        assert_eq!(value["level"], "warn");
        assert_eq!(value["at"], 1_788_000_000_000u64);
        assert_eq!(value["entityId"], "light.desk");
    }

    #[test]
    fn a_log_entry_omits_an_entity_it_does_not_concern() {
        let line = Envelope::new(1, Event::Log {
            level: Level::Info,
            at: 1,
            text: "Connected".into(),
            entity_id: None,
        })
        .to_line();
        assert!(!line.contains("entityId"));
    }

    #[test]
    fn events_are_a_single_line() {
        let line = Envelope::new(1, Event::Log {
            level: Level::Info,
            at: 1,
            text: "a\nb".into(),
            entity_id: None,
        })
        .to_line();
        assert_eq!(line.lines().count(), 1, "a newline in a message must not split the frame");
    }

    #[test]
    fn config_carries_the_area_flags_the_settings_ui_reflects() {
        let line = Envelope::new(1, Event::Config {
            favorites: vec!["light.desk".into()],
            pinned_tabs: vec!["area:office".into()],
            allow_sensitive_ipc: false,
            show_favorites: false,
            selected_tab: "area:office".into(),
            imported_dashboard_prefs: true,
            base_url: "https://ha.example.com".into(),
            hide_empty_areas: true,
            hide_entities_without_area: false,
        })
        .to_line();
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["hideEmptyAreas"], true);
        assert_eq!(value["hideEntitiesWithoutArea"], false);
        assert_eq!(value["showFavorites"], false);
        assert_eq!(value["pinnedTabs"][0], "area:office");
    }

    #[test]
    fn status_omits_absent_fields_and_hides_the_plaintext_flag_when_false() {
        let line = Envelope::new(1, Event::Status {
            state: ConnectionState::Connected,
            message: None,
            origin: Some("https://ha.example.com".into()),
            ha_version: Some("2026.8.3".into()),
            plaintext: false,
        })
        .to_line();
        assert!(!line.contains("message"));
        assert!(!line.contains("plaintext"));
        assert!(line.contains("2026.8.3"));
    }
}
