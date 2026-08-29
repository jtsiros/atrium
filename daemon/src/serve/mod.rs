use std::collections::{HashMap, VecDeque};
use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use crate::config::Config;
use crate::ha::client::Error as ClientError;
use crate::ha::registry::{EntityEntry, Registry};
use crate::ha::url;
use crate::ha::{filter, model};
use crate::protocol::{Command, ConnectionState, Envelope, Event, Level};

const BACKOFF_START: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(60);

// Home Assistant sends nothing while the house is quiet, so silence alone is
// not a fault. A ping after IDLE_PING turns silence into a question; no inbound
// frame at all within IDLE_LIMIT means the socket is half-open.
const IDLE_PING: Duration = Duration::from_secs(30);
const IDLE_LIMIT: Duration = Duration::from_secs(90);
const REGISTRY_REFRESH_FLOOR: Duration = Duration::from_secs(10);

pub struct Daemon {
    config: Config,
    config_path: PathBuf,
    generation: u64,
    registry: Registry,
    entities: HashMap<String, EntityEntry>,
    connected_at: std::time::Instant,
    registry_stale: bool,
    states: HashMap<String, Value>,
    out: mpsc::UnboundedSender<String>,
    // Kept so a panel opened after the fact can still see what happened. Home
    // devices misbehave while nobody is looking at them.
    history: RefCell<VecDeque<Event>>,
}

const HISTORY: usize = 200;

enum Outcome {
    Lost(ClientError),
    Reconfigure,
    Stop,
}

pub async fn run() -> std::io::Result<()> {
    let config_path = Config::path();
    let (config, load_failure) = match Config::load(&config_path) {
        Ok(config) => (config, None),
        Err(message) => (Config::default(), Some(message)),
    };

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
    let writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(line) = out_rx.recv().await {
            if stdout.write_all(line.as_bytes()).await.is_err() || stdout.write_all(b"\n").await.is_err() {
                break;
            }
            let _ = stdout.flush().await;
        }
    });

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    tokio::spawn(async move {
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<Command>(trimmed) {
                Ok(command) => {
                    if cmd_tx.send(command).is_err() {
                        break;
                    }
                }
                Err(e) => eprintln!("atriumd: ignoring unreadable command ({e})"),
            }
        }
    });

    let mut daemon = Daemon {
        config,
        config_path,
        generation: 0,
        registry: Registry::default(),
        entities: HashMap::new(),
        connected_at: std::time::Instant::now(),
        registry_stale: false,
        states: HashMap::new(),
        out: out_tx,
        history: RefCell::new(VecDeque::with_capacity(HISTORY)),
    };
    if let Some(message) = load_failure {
        daemon.error(message);
    }
    daemon.main_loop(cmd_rx).await;
    drop(daemon);
    let _ = writer.await;
    Ok(())
}

mod commands;
mod events;
mod lifecycle;

impl Daemon {
    fn emit(&self, event: Event) {
        let _ = self.out.send(Envelope::new(self.generation, event).to_line());
    }

    fn log(&self, level: Level, text: impl Into<String>, entity_id: Option<String>) {
        let event = Event::Log {
            level,
            at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or_default(),
            text: text.into(),
            entity_id,
        };
        let mut history = self.history.borrow_mut();
        if history.len() == HISTORY {
            history.pop_front();
        }
        history.push_back(event.clone());
        drop(history);
        self.emit(event);
    }

    fn info(&self, text: impl Into<String>) { self.log(Level::Info, text, None) }
    fn warn(&self, text: impl Into<String>) { self.log(Level::Warn, text, None) }
    fn error(&self, text: impl Into<String>) { self.log(Level::Error, text, None) }

    fn replay_history(&self) {
        for event in self.history.borrow().iter() {
            self.emit(event.clone());
        }
    }

    fn emit_status(&self, state: ConnectionState, message: Option<String>) {
        let endpoint = url::parse(&self.config.base_url).ok();
        self.emit(Event::Status {
            state,
            message,
            origin: endpoint.as_ref().map(|e| e.origin.clone()),
            ha_version: None,
            plaintext: endpoint.as_ref().is_some_and(|e| e.plaintext),
        });
    }

    fn emit_config(&self) {
        self.emit(Event::Config {
            favorites: self.config.favorites.clone(),
            pinned_tabs: self.config.pinned_tabs.clone(),
            allow_sensitive_ipc: self.config.allow_sensitive_ipc,
            show_favorites: self.config.show_favorites,
            selected_tab: self.config.selected_tab.clone(),
            imported_dashboard_prefs: self.config.imported_dashboard_prefs,
            base_url: self.config.base_url.clone(),
            hide_empty_areas: self.config.areas.hide_empty_areas,
            hide_entities_without_area: self.config.areas.hide_entities_without_area,
        });
    }

    fn save(&self) {
        if let Err(e) = self.config.save(&self.config_path) {
            self.error(format!("Settings could not be saved: {e}"));
        }
    }

    fn display_name(&self, entity_id: &str) -> String {
        if let Some(override_name) = self.config.display_name_overrides.get(entity_id) {
            if !override_name.is_empty() {
                return override_name.clone();
            }
        }
        self.states
            .get(entity_id)
            .and_then(|s| s.get("attributes")?.get("friendly_name")?.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| entity_id.to_string())
    }

    fn registry_name(&self, entity_id: &str) -> Option<String> {
        self.config
            .display_name_overrides
            .get(entity_id)
            .filter(|n| !n.is_empty())
            .cloned()
            .or_else(|| self.entities.get(entity_id).and_then(|e| e.name.clone()))
    }

    fn registry_icon(&self, entity_id: &str) -> Option<String> {
        self.entities.get(entity_id).and_then(|e| e.icon.clone())
    }

    // The tab projection filters; the row stream has to filter identically, or
    // entities the user hid in Home Assistant reach the panel and become
    // actionable even though no tab ever draws them.
    fn visible(&self, entity_id: &str) -> bool {
        filter::is_visible(entity_id, self.entities.get(entity_id))
    }

    fn emit_projection(&self) {
        let mut live: Vec<String> = self.states.keys().cloned().collect();
        live.sort();
        let mut tabs = self
            .registry
            .project_tabs(&live, &self.config.areas, |id| self.display_name(id));
        if let Some(pinned) = self.pinned_tab() {
            tabs.insert(0, pinned);
        }
        self.emit(Event::Tabs { tabs });
        self.emit(Event::Areas {
            areas: self.registry.area_choices(&live, &self.config.areas),
        });
    }

    // Pinned entities are a tab in front of the rooms. It only exists once the
    // user has both asked for it and pinned something that is actually visible.
    fn pinned_tab(&self) -> Option<crate::ha::registry::Tab> {
        if !self.config.show_favorites {
            return None;
        }
        let entity_ids: Vec<String> = self
            .config
            .favorites
            .iter()
            .filter(|id| self.states.contains_key(*id) && self.visible(id))
            .cloned()
            .collect();
        if entity_ids.is_empty() {
            return None;
        }
        Some(crate::ha::registry::Tab {
            id: "pinned".into(),
            title: "Pinned".into(),
            icon: "mdi:star".into(),
            glyph: crate::ha::icons::glyph("mdi:star").map(String::from).unwrap_or_default(),
            entity_ids,
        })
    }

    fn emit_all_rows(&self) {
        let mut rows: Vec<model::Row> = self
            .states
            .values()
            .filter_map(|entity| {
                let id = entity.get("entity_id")?.as_str()?;
                if !self.visible(id) {
                    return None;
                }
                model::row(
                    entity,
                    self.registry_name(id).as_deref(),
                    self.registry_icon(id).as_deref(),
                )
            })
            .collect();
        rows.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
        self.emit(Event::Rows { rows });
    }

}
