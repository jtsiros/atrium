use serde_json::Value;

use super::{Daemon, REGISTRY_REFRESH_FLOOR};
use crate::ha::{model, text};
use crate::protocol::{Event, Level};

impl Daemon {
    pub(super) fn handle_event(&mut self, message: &Value) {
        if message.get("type").and_then(Value::as_str) != Some("event") {
            if message.get("success").and_then(Value::as_bool) == Some(false) {
                let text = message
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Home Assistant refused the request");
                self.log(Level::Warn, text, None);
            }
            return;
        }
        let Some(event) = message.get("event") else { return };
        match event.get("event_type").and_then(Value::as_str) {
            Some("state_changed") => self.handle_state_changed(event),
            // Re-projecting would use the registry fetched at connect time, so a
            // renamed or added area would never appear. Reconnecting refetches
            // it; the guard keeps an integration reload from looping.
            Some("area_registry_updated" | "entity_registry_updated" | "device_registry_updated")
                if self.connected_at.elapsed() >= REGISTRY_REFRESH_FLOOR =>
            {
                self.info("Home Assistant areas or devices changed; reloading.");
                self.registry_stale = true;
            }
            _ => {}
        }
    }

    fn handle_state_changed(&mut self, event: &Value) {
        let Some(data) = event.get("data") else { return };
        let Some(entity_id) = data.get("entity_id").and_then(Value::as_str) else {
            return;
        };
        // Ingested ids become map keys on both sides of the protocol, so a
        // server-supplied "__proto__" or "constructor" must never get that far.
        if !text::valid_entity_id(entity_id) {
            return;
        }
        if !self.visible(entity_id) {
            return;
        }

        let Some(new_state) = data.get("new_state") else { return };
        if new_state.is_null() {
            self.states.remove(entity_id);
            self.emit(Event::Dropped { entity_id: entity_id.to_string() });
            self.emit_projection();
            return;
        }

        let is_new = !self.states.contains_key(entity_id);
        self.states.insert(entity_id.to_string(), new_state.clone());

        if let Some(row) = model::row(
            new_state,
            self.registry_name(entity_id).as_deref(),
            self.registry_icon(entity_id).as_deref(),
        ) {
            self.emit(Event::Row { row });
        }
        if is_new {
            self.emit_projection();
        }
    }
}
