use tokio::sync::mpsc;

use super::{Daemon, Outcome, BACKOFF_MAX, BACKOFF_START, IDLE_LIMIT, IDLE_PING};
use crate::protocol::Level;
use crate::ha::client::{CommandWriter, Error as ClientError, EventReader, Session};
use crate::ha::url::{self, Endpoint};
use crate::ha::prefs;
use crate::keyring;
use crate::protocol::{Command, ConnectionState, Event};

impl Daemon {
    pub(super) async fn main_loop(&mut self, mut commands: mpsc::UnboundedReceiver<Command>) {
        let mut backoff = BACKOFF_START;
        self.emit_config();

        loop {
            let endpoint = match url::parse(&self.config.base_url) {
                Ok(endpoint) => endpoint,
                Err(_) => {
                    self.emit_status(ConnectionState::Unconfigured, None);
                    match self.idle(&mut commands).await {
                        Outcome::Stop => return,
                        _ => continue,
                    }
                }
            };

            let token = match keyring::lookup(&endpoint.origin).await {
                Ok(Some(token)) => token,
                Ok(None) => {
                    self.emit_status(ConnectionState::NeedsToken, None);
                    match self.idle(&mut commands).await {
                        Outcome::Stop => return,
                        _ => continue,
                    }
                }
                Err(e) => {
                    self.emit_status(ConnectionState::Failed, Some(e.to_string()));
                    match self.idle(&mut commands).await {
                        Outcome::Stop => return,
                        _ => continue,
                    }
                }
            };

            self.generation += 1;
            self.emit_status(ConnectionState::Connecting, None);
            self.info(format!("Connecting to {}", endpoint.origin));

            match self.connect_once(&endpoint, &token, &mut commands).await {
                Outcome::Stop => return,
                Outcome::Reconfigure => {
                    backoff = BACKOFF_START;
                    continue;
                }
                Outcome::Lost(e) => {
                    if !e.is_retryable() {
                        self.error(format!("{e}. Enter a new token to try again."));
                        self.emit_status(ConnectionState::Failed, Some(e.to_string()));
                        // Retrying a rejected token only produces failed logins,
                        // which Home Assistant answers by banning the address.
                        // Wait for the credential to actually change.
                        match self.idle_until_credentials_change(&mut commands).await {
                            Outcome::Stop => return,
                            _ => continue,
                        }
                    }
                    self.warn(format!("{e}. Retrying in {}s.", backoff.as_secs()));
                    self.emit_status(ConnectionState::Reconnecting, Some(e.to_string()));
                    tokio::select! {
                        _ = tokio::time::sleep(backoff) => {}
                        command = commands.recv() => match command {
                            None => return,
                            Some(command) => {
                                if matches!(self.handle_offline(command).await, Outcome::Stop) {
                                    return;
                                }
                                backoff = BACKOFF_START;
                                continue;
                            }
                        }
                    }
                    backoff = (backoff * 2).min(BACKOFF_MAX);
                }
            }
        }
    }

    async fn idle_until_credentials_change(
        &mut self,
        commands: &mut mpsc::UnboundedReceiver<Command>,
    ) -> Outcome {
        loop {
            match commands.recv().await {
                None => return Outcome::Stop,
                Some(command) => {
                    let credential_change = matches!(
                        command,
                        Command::SetToken { .. } | Command::SetUrl { .. } | Command::ForgetToken
                    );
                    match self.handle_offline(command).await {
                        Outcome::Stop => return Outcome::Stop,
                        Outcome::Reconfigure if credential_change => return Outcome::Reconfigure,
                        _ => continue,
                    }
                }
            }
        }
    }

    async fn idle(&mut self, commands: &mut mpsc::UnboundedReceiver<Command>) -> Outcome {
        loop {
            match commands.recv().await {
                None => return Outcome::Stop,
                Some(command) => match self.handle_offline(command).await {
                    Outcome::Stop => return Outcome::Stop,
                    Outcome::Reconfigure => return Outcome::Reconfigure,
                    Outcome::Lost(_) => continue,
                },
            }
        }
    }

    async fn connect_once(
        &mut self,
        endpoint: &Endpoint,
        token: &str,
        commands: &mut mpsc::UnboundedReceiver<Command>,
    ) -> Outcome {
        let mut session = match Session::connect(endpoint, token).await {
            Ok(session) => session,
            Err(e) => return Outcome::Lost(e),
        };
        let ha_version = session.ha_version.clone();

        let snapshot = match session.snapshot().await {
            Ok(snapshot) => snapshot,
            Err(e) => return Outcome::Lost(e),
        };

        if !self.config.imported_dashboard_prefs {
            let current_hide_empty = self.config.areas.hide_empty_areas;
            if let Some(imported) = session
                .lovelace_config()
                .await
                .as_ref()
                .and_then(|config| prefs::from_lovelace_config(config, current_hide_empty))
            {
                self.config.areas = imported;
                self.config.imported_dashboard_prefs = true;
                self.save();
                self.emit_config();
            }
        }

        for event_type in ["state_changed", "area_registry_updated", "entity_registry_updated", "device_registry_updated"] {
            if let Err(e) = session.subscribe(event_type).await {
                return Outcome::Lost(e);
            }
        }

        self.entities = snapshot
            .registry
            .entities
            .iter()
            .map(|entry| (entry.entity_id.clone(), entry.clone()))
            .collect();
        self.registry = snapshot.registry;
        self.states = snapshot
            .states
            .into_iter()
            .filter_map(|entity| {
                let id = entity.get("entity_id")?.as_str()?.to_string();
                Some((id, entity))
            })
            .collect();

        self.emit(Event::Status {
            state: ConnectionState::Connected,
            message: None,
            origin: Some(endpoint.origin.clone()),
            ha_version: Some(ha_version.clone()),
            plaintext: endpoint.plaintext,
        });
        self.connected_at = std::time::Instant::now();
        self.registry_stale = false;
        self.log(
            Level::Info,
            format!(
                "Connected to Home Assistant {} — {} areas, {} devices, {} entities",
                ha_version,
                self.registry.areas.len(),
                self.registry.devices.len(),
                self.states.len()
            ),
            None,
        );
        self.emit_projection();
        self.emit_all_rows();

        let (mut writer, mut reader) = session.into_parts();
        let outcome = self.stream(&mut writer, &mut reader, commands).await;
        writer.close().await;
        outcome
    }

    async fn stream(
        &mut self,
        writer: &mut CommandWriter,
        reader: &mut EventReader,
        commands: &mut mpsc::UnboundedReceiver<Command>,
    ) -> Outcome {
        let mut last_frame = std::time::Instant::now();
        loop {
            tokio::select! {
                event = tokio::time::timeout(IDLE_PING, reader.next_event()) => match event {
                    Ok(Ok(event)) => {
                        last_frame = std::time::Instant::now();
                        self.handle_event(&event);
                        if self.registry_stale {
                            return Outcome::Reconfigure;
                        }
                    }
                    Ok(Err(e)) => return Outcome::Lost(e),
                    Err(_) => {
                        if last_frame.elapsed() >= IDLE_LIMIT {
                            return Outcome::Lost(ClientError::Timeout("waiting for Home Assistant"));
                        }
                        if let Err(e) = writer.ping().await {
                            return Outcome::Lost(e);
                        }
                    }
                },
                command = commands.recv() => match command {
                    None => return Outcome::Stop,
                    Some(command) => {
                        if let Some(outcome) = self.handle_online(command, writer).await {
                            return outcome;
                        }
                    }
                },
            }
        }
    }

}
