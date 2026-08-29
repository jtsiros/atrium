use serde::Serialize;
use serde_json::Value;

use crate::ha::filter::domain_of;
use crate::ha::{device_icons, icons, text};

mod features {
    pub mod media_player {
        pub const PAUSE: u64 = 1;
        pub const PREVIOUS: u64 = 1 << 4;
        pub const NEXT: u64 = 1 << 5;
        pub const VOLUME_SET: u64 = 1 << 2;
        pub const VOLUME_MUTE: u64 = 1 << 3;
        pub const PLAY: u64 = 1 << 14;
    }
    pub mod cover {
        pub const OPEN: u64 = 1;
        pub const CLOSE: u64 = 1 << 1;
        pub const SET_POSITION: u64 = 1 << 2;
        pub const STOP: u64 = 1 << 3;
    }
    pub mod climate {
        pub const TARGET_TEMPERATURE: u64 = 1;
        pub const TARGET_TEMPERATURE_RANGE: u64 = 1 << 1;
        pub const FAN_MODE: u64 = 1 << 3;
        pub const PRESET_MODE: u64 = 1 << 4;
    }
    pub mod fan {
        pub const SET_SPEED: u64 = 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Control {
    Toggle,
    Brightness,
    ColorTemp,
    Color,
    Activate,
    Lock,
    OpenClose,
    Position,
    Stop,
    Transport,
    Volume,
    HvacMode,
    Temperature,
    TemperatureRange,
    FanMode,
    PresetMode,
    FanSpeed,
    ReadOnly,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub entity_id: String,
    pub domain: String,
    pub name: String,
    pub icon: String,
    pub glyph: String,
    pub state: String,
    pub display_state: String,
    pub active: bool,
    pub unavailable: bool,
    pub controls: Vec<Control>,
    pub attributes: serde_json::Map<String, Value>,
}

fn attr<'a>(entity: &'a Value, key: &str) -> Option<&'a Value> {
    entity.get("attributes")?.get(key)
}

fn attr_u64(entity: &Value, key: &str) -> u64 {
    attr(entity, key).and_then(Value::as_u64).unwrap_or(0)
}

fn attr_str<'a>(entity: &'a Value, key: &str) -> Option<&'a str> {
    attr(entity, key).and_then(Value::as_str)
}

fn has(bits: u64, flag: u64) -> bool {
    bits & flag != 0
}

fn is_active(domain: &str, state: &str) -> bool {
    match domain {
        "lock" => state == "unlocked",
        "cover" => matches!(state, "open" | "opening"),
        "climate" => state != "off",
        "media_player" => matches!(state, "playing" | "on" | "buffering"),
        _ => state == "on",
    }
}

pub fn capabilities(entity: &Value) -> Vec<Control> {
    let Some(entity_id) = entity.get("entity_id").and_then(Value::as_str) else {
        return vec![Control::ReadOnly];
    };
    let domain = domain_of(entity_id);
    let bits = attr_u64(entity, "supported_features");
    let mut controls = Vec::new();

    match domain {
        "light" => {
            controls.push(Control::Toggle);
            let modes: Vec<&str> = attr(entity, "supported_color_modes")
                .and_then(Value::as_array)
                .map(|m| m.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            if modes.iter().any(|m| *m != "onoff") {
                controls.push(Control::Brightness);
            }
            if modes.contains(&"color_temp") {
                controls.push(Control::ColorTemp);
            }
            if modes
                .iter()
                .any(|m| matches!(*m, "hs" | "rgb" | "rgbw" | "rgbww" | "xy"))
            {
                controls.push(Control::Color);
            }
        }
        "switch" | "input_boolean" | "humidifier" | "siren" | "remote" => {
            controls.push(Control::Toggle)
        }
        "fan" => {
            controls.push(Control::Toggle);
            if has(bits, features::fan::SET_SPEED) {
                controls.push(Control::FanSpeed);
            }
        }
        "lock" => controls.push(Control::Lock),
        "scene" | "script" | "button" | "input_button" => controls.push(Control::Activate),
        "cover" => {
            if has(bits, features::cover::OPEN) || has(bits, features::cover::CLOSE) {
                controls.push(Control::OpenClose);
            }
            if has(bits, features::cover::STOP) {
                controls.push(Control::Stop);
            }
            if has(bits, features::cover::SET_POSITION) {
                controls.push(Control::Position);
            }
        }
        "media_player" => {
            if has(bits, features::media_player::PLAY)
                || has(bits, features::media_player::PAUSE)
                || has(bits, features::media_player::NEXT)
                || has(bits, features::media_player::PREVIOUS)
            {
                controls.push(Control::Transport);
            }
            if has(bits, features::media_player::VOLUME_SET)
                || has(bits, features::media_player::VOLUME_MUTE)
            {
                controls.push(Control::Volume);
            }
        }
        "climate" => {
            controls.push(Control::HvacMode);
            if has(bits, features::climate::TARGET_TEMPERATURE) {
                controls.push(Control::Temperature);
            }
            if has(bits, features::climate::TARGET_TEMPERATURE_RANGE) {
                controls.push(Control::TemperatureRange);
            }
            if has(bits, features::climate::FAN_MODE) {
                controls.push(Control::FanMode);
            }
            if has(bits, features::climate::PRESET_MODE) {
                controls.push(Control::PresetMode);
            }
        }
        _ => {}
    }

    if controls.is_empty() {
        controls.push(Control::ReadOnly);
    }
    controls
}

// Allow-listed per control: an entity's full attribute bag can carry camera
// access tokens, signed media URLs and GPS coordinates, and rows reach both the
// panel and debug output.
fn needed_attributes(controls: &[Control], entity: &Value) -> serde_json::Map<String, Value> {
    let mut keys: Vec<&str> = vec!["friendly_name", "device_class", "unit_of_measurement"];
    for control in controls {
        keys.extend(match control {
            Control::Brightness => &["brightness"][..],
            Control::ColorTemp => &["color_temp_kelvin", "min_color_temp_kelvin", "max_color_temp_kelvin"],
            Control::Color => &["hs_color", "rgb_color"],
            Control::Position => &["current_position"],
            Control::Transport => &["media_title", "media_artist"],
            Control::Volume => &["volume_level", "is_volume_muted"],
            Control::HvacMode => &["hvac_modes", "hvac_action", "current_temperature"],
            Control::Temperature => &["temperature", "min_temp", "max_temp", "target_temp_step"],
            Control::TemperatureRange => &["target_temp_low", "target_temp_high", "min_temp", "max_temp"],
            Control::FanMode => &["fan_mode", "fan_modes"],
            Control::PresetMode => &["preset_mode", "preset_modes"],
            Control::FanSpeed => &["percentage", "percentage_step"],
            _ => &[],
        });
    }

    let mut out = serde_json::Map::new();
    let Some(Value::Object(attributes)) = entity.get("attributes") else {
        return out;
    };
    for key in keys {
        if let Some(value) = attributes.get(key) {
            out.insert(key.to_string(), value.clone());
        }
    }
    out
}

fn display_state(entity: &Value, domain: &str, state: &str) -> String {
    if matches!(state, "unavailable" | "unknown") {
        return "Unavailable".into();
    }
    if let Some(unit) = attr_str(entity, "unit_of_measurement") {
        return format!("{state} {unit}");
    }
    match (domain, state) {
        ("lock", "locked") => "Locked".into(),
        ("lock", "unlocked") => "Unlocked".into(),
        (_, "on") => "On".into(),
        (_, "off") => "Off".into(),
        _ => {
            let mut chars = state.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().replace('_', " "),
                None => String::new(),
            }
        }
    }
}

fn default_icon(domain: &str, device_class: Option<&str>, active: bool) -> &'static str {
    let by_class = match domain {
        "sensor" => device_icons::for_sensor(device_class),
        "binary_sensor" => device_icons::for_binary_sensor(device_class, active),
        "switch" => device_icons::for_switch(device_class, active),
        "cover" => device_icons::for_cover(device_class, active),
        "media_player" => device_icons::for_media_player(device_class),
        _ => None,
    };
    if let Some(icon) = by_class {
        return icon;
    }

    match domain {
        "light" => {
            if active {
                "mdi:lightbulb"
            } else {
                "mdi:lightbulb-off"
            }
        }
        "switch" => "mdi:toggle-switch-outline",
        "fan" => "mdi:fan",
        "lock" => "mdi:lock",
        "cover" => "mdi:window-shutter",
        "climate" => "mdi:thermostat",
        "media_player" => "mdi:speaker",
        "scene" => "mdi:palette",
        "script" => "mdi:script-text",
        "button" | "input_button" => "mdi:gesture-tap-button",
        "binary_sensor" => {
            if active {
                "mdi:checkbox-marked-circle"
            } else {
                "mdi:checkbox-blank-circle-outline"
            }
        }
        "sensor" => "mdi:eye",
        "humidifier" => "mdi:air-humidifier",
        "remote" => "mdi:remote",
        _ => "mdi:help-circle-outline",
    }
}

pub fn row(entity: &Value, registry_name: Option<&str>, registry_icon: Option<&str>) -> Option<Row> {
    let entity_id = entity.get("entity_id")?.as_str()?.to_string();
    let domain = domain_of(&entity_id).to_string();
    let state = entity.get("state").and_then(Value::as_str).unwrap_or("unknown").to_string();
    let active = is_active(&domain, &state);
    let unavailable = matches!(state.as_str(), "unavailable" | "unknown");

    let name = registry_name
        .map(text::plain)
        .filter(|n| !n.is_empty())
        .or_else(|| attr_str(entity, "friendly_name").map(text::plain))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| entity_id.clone());

    let device_class = attr_str(entity, "device_class");

    let icon = registry_icon
        .filter(|i| !i.is_empty())
        .map(str::to_string)
        .or_else(|| attr_str(entity, "icon").map(str::to_string))
        .unwrap_or_else(|| default_icon(&domain, device_class, active).to_string());

    let controls = capabilities(entity);
    let attributes = needed_attributes(&controls, entity);

    let glyph = icons::glyph(&icon)
        .or_else(|| icons::glyph(default_icon(&domain, device_class, active)))
        .map(String::from)
        .unwrap_or_default();

    Some(Row {
        display_state: text::plain(&display_state(entity, &domain, &state)),
        glyph,
        entity_id,
        domain,
        name,
        icon,
        state,
        active,
        unavailable,
        controls,
        attributes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entity(id: &str, state: &str, attributes: Value) -> Value {
        json!({ "entity_id": id, "state": state, "attributes": attributes })
    }

    #[test]
    fn a_dimmable_colour_light_offers_every_light_control() {
        let e = entity(
            "light.desk",
            "on",
            json!({ "supported_color_modes": ["color_temp", "hs"], "brightness": 180 }),
        );
        let controls = capabilities(&e);
        assert!(controls.contains(&Control::Toggle));
        assert!(controls.contains(&Control::Brightness));
        assert!(controls.contains(&Control::ColorTemp));
        assert!(controls.contains(&Control::Color));
    }

    #[test]
    fn an_onoff_only_light_offers_no_brightness() {
        let e = entity("light.porch", "off", json!({ "supported_color_modes": ["onoff"] }));
        let controls = capabilities(&e);
        assert_eq!(controls, vec![Control::Toggle]);
    }

    #[test]
    fn media_player_features_are_read_from_the_bitmask() {
        let e = entity("media_player.tv", "playing", json!({ "supported_features": 16384 | 4 }));
        let controls = capabilities(&e);
        assert!(controls.contains(&Control::Transport));
        assert!(controls.contains(&Control::Volume));

        let bare = entity("media_player.dumb", "idle", json!({ "supported_features": 0 }));
        assert_eq!(capabilities(&bare), vec![Control::ReadOnly]);
    }

    #[test]
    fn a_cover_without_position_gets_no_position_slider() {
        let e = entity("cover.door", "open", json!({ "supported_features": 1 | 2 | 8 }));
        let controls = capabilities(&e);
        assert!(controls.contains(&Control::OpenClose));
        assert!(controls.contains(&Control::Stop));
        assert!(!controls.contains(&Control::Position));
    }

    #[test]
    fn remotes_can_be_turned_on_and_off() {
        let e = entity("remote.living_room_apple_tv", "on", json!({}));
        assert_eq!(capabilities(&e), vec![Control::Toggle]);
    }

    #[test]
    fn sensors_are_read_only() {
        let e = entity("sensor.temp", "21.5", json!({ "unit_of_measurement": "°C" }));
        assert_eq!(capabilities(&e), vec![Control::ReadOnly]);
    }

    #[test]
    fn active_state_follows_the_domains_own_meaning() {
        assert!(!row(&entity("lock.front", "locked", json!({})), None, None).unwrap().active);
        assert!(row(&entity("lock.front", "unlocked", json!({})), None, None).unwrap().active);
        assert!(row(&entity("cover.door", "open", json!({})), None, None).unwrap().active);
        assert!(row(&entity("climate.hall", "heat", json!({})), None, None).unwrap().active);
        assert!(!row(&entity("climate.hall", "off", json!({})), None, None).unwrap().active);
    }

    #[test]
    fn sensitive_attributes_never_reach_a_row() {
        let e = entity(
            "camera.front_door",
            "idle",
            json!({
                "friendly_name": "Front Door",
                "access_token": "sekrit",
                "entity_picture": "/api/camera_proxy/camera.front_door?token=sekrit",
                "latitude": 51.5,
                "longitude": -0.1
            }),
        );
        let row = row(&e, None, None).unwrap();
        for banned in ["access_token", "entity_picture", "latitude", "longitude"] {
            assert!(!row.attributes.contains_key(banned), "{banned} leaked into the row");
        }
        assert_eq!(row.attributes.get("friendly_name").unwrap(), "Front Door");
    }

    #[test]
    fn only_attributes_the_controls_need_are_carried() {
        let e = entity(
            "light.desk",
            "on",
            json!({ "supported_color_modes": ["onoff"], "brightness": 200, "hs_color": [0, 0] }),
        );
        let row = row(&e, None, None).unwrap();
        assert!(!row.attributes.contains_key("brightness"));
        assert!(!row.attributes.contains_key("hs_color"));
    }

    #[test]
    fn server_controlled_names_cannot_carry_markup() {
        let e = entity("light.a", "on", json!({ "friendly_name": "<img src=x>Desk" }));
        assert_eq!(row(&e, None, None).unwrap().name, "img src=xDesk");

        let e = entity("sensor.a", "21", json!({ "unit_of_measurement": "<b>C" }));
        assert!(!row(&e, None, None).unwrap().display_state.contains('<'));
    }

    #[test]
    fn names_prefer_the_registry_override_then_the_friendly_name() {
        let e = entity("light.desk", "on", json!({ "friendly_name": "Desk Lamp" }));
        assert_eq!(row(&e, Some("Reading Light"), None).unwrap().name, "Reading Light");
        assert_eq!(row(&e, None, None).unwrap().name, "Desk Lamp");

        let bare = entity("light.nameless", "on", json!({}));
        assert_eq!(row(&bare, None, None).unwrap().name, "light.nameless");
    }

    #[test]
    fn a_device_class_picks_the_icon_the_dashboard_shows() {
        let power = entity("sensor.plug_power", "18", json!({ "device_class": "power" }));
        assert_eq!(row(&power, None, None).unwrap().icon, "mdi:flash");

        let energy = entity("sensor.plug_energy", "4.2", json!({ "device_class": "energy" }));
        assert_eq!(row(&energy, None, None).unwrap().icon, "mdi:lightning-bolt");

        let temp = entity("sensor.hall", "21", json!({ "device_class": "temperature" }));
        assert_eq!(row(&temp, None, None).unwrap().icon, "mdi:thermometer");

        // an outlet reads its state, a plain sensor does not
        let on = entity("switch.plug", "on", json!({ "device_class": "outlet" }));
        let off = entity("switch.plug", "off", json!({ "device_class": "outlet" }));
        assert_eq!(row(&on, None, None).unwrap().icon, "mdi:power-plug");
        assert_eq!(row(&off, None, None).unwrap().icon, "mdi:power-plug-off");

        // an entity's own icon still wins over the class
        let explicit = entity("sensor.plug_power", "18",
                              json!({ "device_class": "power", "icon": "mdi:desk" }));
        assert_eq!(row(&explicit, None, None).unwrap().icon, "mdi:desk");
    }

    #[test]
    fn binary_sensors_use_home_assistants_own_pair() {
        let on = row(&entity("binary_sensor.door", "on", json!({})), None, None).unwrap();
        let off = row(&entity("binary_sensor.door", "off", json!({})), None, None).unwrap();
        assert_eq!(on.icon, "mdi:checkbox-marked-circle");
        assert_eq!(off.icon, "mdi:checkbox-blank-circle-outline");
        assert!(!on.glyph.is_empty() && !off.glyph.is_empty());
    }

    #[test]
    fn icons_fall_back_by_domain_and_track_light_state() {
        let on = row(&entity("light.a", "on", json!({})), None, None).unwrap();
        let off = row(&entity("light.a", "off", json!({})), None, None).unwrap();
        assert_eq!(on.icon, "mdi:lightbulb");
        assert_eq!(off.icon, "mdi:lightbulb-off");

        let custom = row(&entity("light.a", "on", json!({ "icon": "mdi:desk" })), None, None).unwrap();
        assert_eq!(custom.icon, "mdi:desk");
    }

    #[test]
    fn wire_names_the_panel_reads_are_camel_case() {
        let row = row(&entity("light.desk", "on", json!({})), None, None).unwrap();
        let json = serde_json::to_value(&row).unwrap();
        for key in ["entityId", "domain", "name", "icon", "glyph", "state",
                    "displayState", "active", "unavailable", "controls", "attributes"] {
            assert!(json.get(key).is_some(), "Row is missing {key}: {json}");
        }
    }

    #[test]
    fn every_row_carries_a_glyph_to_draw() {
        let known = row(&entity("light.a", "on", json!({ "icon": "mdi:desk-lamp" })), None, None).unwrap();
        assert!(!known.glyph.is_empty());

        let unknown = row(&entity("light.a", "on", json!({ "icon": "mdi:not-a-real-icon" })), None, None).unwrap();
        assert!(!unknown.glyph.is_empty(), "an unknown icon must fall back, not blank out");

        let bare = row(&entity("sensor.x", "1", json!({})), None, None).unwrap();
        assert!(!bare.glyph.is_empty());
    }

    #[test]
    fn unavailable_entities_are_marked_and_read_plainly() {
        let row = row(&entity("light.gone", "unavailable", json!({})), None, None).unwrap();
        assert!(row.unavailable);
        assert!(!row.active);
        assert_eq!(row.display_state, "Unavailable");
    }

    #[test]
    fn display_state_carries_units_and_titlecases_enums() {
        let sensor = row(&entity("sensor.t", "21.5", json!({ "unit_of_measurement": "°C" })), None, None).unwrap();
        assert_eq!(sensor.display_state, "21.5 °C");

        let climate = row(&entity("climate.h", "heat_cool", json!({})), None, None).unwrap();
        assert_eq!(climate.display_state, "Heat cool");

        let light = row(&entity("light.a", "on", json!({})), None, None).unwrap();
        assert_eq!(light.display_state, "On");
    }
}
