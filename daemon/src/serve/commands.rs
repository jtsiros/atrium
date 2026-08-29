use super::{Daemon, Outcome};
use crate::ha::client::{CommandWriter, Error as ClientError};
use crate::ha::action;
use crate::ha::url;
use crate::keyring;
use crate::protocol::{Command, ConnectionState, Level};

impl Daemon {
    pub(super) async fn handle_offline(&mut self, command: Command) -> Outcome {
        match command {
            Command::Connect | Command::Refresh => Outcome::Reconfigure,
            Command::Disconnect => {
                self.emit_status(ConnectionState::Offline, None);
                Outcome::Lost(ClientError::Closed)
            }
            Command::SetUrl { url: new_url } => {
                if let Err(e) = url::parse(&new_url) {
                    self.warn(e.to_string());
                    return Outcome::Lost(ClientError::Closed);
                }
                self.config.base_url = new_url;
                self.save();
                self.emit_config();
                Outcome::Reconfigure
            }
            Command::SetToken { token, url } => {
                // The address is resolved from this command, never from stored
                // config: a token typed for one server must never be stored
                // under, or sent to, the server that happens to be configured.
                let target = url.as_deref().unwrap_or(&self.config.base_url);
                let endpoint = match url::parse(target) {
                    Ok(endpoint) => endpoint,
                    Err(e) => {
                        self.error(e.to_string());
                        return Outcome::Lost(ClientError::Closed);
                    }
                };
                let token = token.trim();
                if token.is_empty() {
                    self.warn("That token is empty.");
                    return Outcome::Lost(ClientError::Closed);
                }
                match keyring::store(&endpoint.origin, token).await {
                    Ok(()) => {
                        if self.config.base_url != target {
                            self.config.base_url = target.to_string();
                            self.save();
                            self.emit_config();
                        }
                        Outcome::Reconfigure
                    }
                    Err(e) => {
                        self.error(e.to_string());
                        Outcome::Lost(ClientError::Closed)
                    }
                }
            }
            Command::ForgetToken => {
                if let Ok(endpoint) = url::parse(&self.config.base_url) {
                    if let Err(e) = keyring::clear(&endpoint.origin).await {
                        self.error(e.to_string());
                    }
                }
                Outcome::Reconfigure
            }
            Command::SetAreaPrefs { order, hidden, hide_empty_areas, hide_entities_without_area } => {
                if let Some(order) = order {
                    self.config.areas.order = order;
                }
                if let Some(hidden) = hidden {
                    self.config.areas.hidden = hidden;
                }
                if let Some(value) = hide_empty_areas {
                    self.config.areas.hide_empty_areas = value;
                }
                if let Some(value) = hide_entities_without_area {
                    self.config.areas.hide_entities_without_area = value;
                }
                self.config.imported_dashboard_prefs = true;
                self.save();
                self.emit_projection();
                self.emit_config();
                Outcome::Lost(ClientError::Closed)
            }
            Command::SetFavorites { ids, show } => {
                self.config.favorites = ids;
                if let Some(value) = show {
                    self.config.show_favorites = value;
                }
                self.save();
                self.emit_config();
                Outcome::Lost(ClientError::Closed)
            }
            Command::SetPinnedTabs { ids } => {
                self.config.pinned_tabs = ids;
                self.save();
                self.emit_config();
                Outcome::Lost(ClientError::Closed)
            }
            Command::SetSelectedTab { tab } => {
                self.config.selected_tab = tab;
                self.save();
                Outcome::Lost(ClientError::Closed)
            }
            Command::Status => {
                self.emit_config();
                self.emit_projection();
                self.replay_history();
                Outcome::Lost(ClientError::Closed)
            }
            Command::Action { entity_id, .. } => {
                self.log(Level::Warn, "Not connected to Home Assistant.", Some(entity_id));
                Outcome::Lost(ClientError::Closed)
            }
            Command::ImportDashboardPrefs => {
                self.warn("Not connected to Home Assistant.");
                Outcome::Lost(ClientError::Closed)
            }
        }
    }

    pub(super) async fn handle_online(&mut self, command: Command, writer: &mut CommandWriter) -> Option<Outcome> {
        match command {
            Command::Action { entity_id, action, data } => {
                match action::resolve(&entity_id, &action, &data) {
                    Ok(resolved) => {
                        if let Err(e) = writer
                            .call_service(&resolved.domain, &resolved.service, &resolved.entity_id, resolved.data)
                            .await
                        {
                            return Some(Outcome::Lost(e));
                        }
                        self.log(
                            Level::Info,
                            format!(
                                "{} — {}.{}",
                                self.display_name(&entity_id),
                                resolved.domain,
                                resolved.service
                            ),
                            Some(entity_id.clone()),
                        );
                    }
                    Err(e) => self.log(Level::Warn, format!("{e}"), Some(entity_id.clone())),
                }
                None
            }
            Command::Refresh => Some(Outcome::Reconfigure),
            Command::Disconnect => {
                self.emit_status(ConnectionState::Offline, None);
                Some(Outcome::Stop)
            }
            Command::ImportDashboardPrefs => {
                self.info("Re-reading your Home Assistant dashboard.");
                self.config.imported_dashboard_prefs = false;
                self.save();
                Some(Outcome::Reconfigure)
            }
            Command::Status => {
                self.emit_config();
                self.emit_projection();
                self.replay_history();
                None
            }
            other => match self.handle_offline(other).await {
                Outcome::Stop => Some(Outcome::Stop),
                Outcome::Reconfigure => Some(Outcome::Reconfigure),
                Outcome::Lost(_) => None,
            },
        }
    }

}
