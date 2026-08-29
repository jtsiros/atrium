use serde_json::{json, Map, Value};

use crate::ha::filter::domain_of;
use crate::ha::text::valid_entity_id;

// Wide enough for either unit and any thermostat, narrow enough that a bad
// value cannot be forwarded to the equipment.
const TEMPERATURE_RANGE: std::ops::RangeInclusive<f64> = -50.0..=150.0;

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceCall {
    pub domain: String,
    pub service: String,
    pub entity_id: String,
    pub data: Value,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    BadEntityId,
    Unsupported { action: String, domain: String },
    BadArgument(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadEntityId => write!(f, "not a valid entity id"),
            Self::Unsupported { action, domain } => {
                write!(f, "“{action}” is not something a {domain} entity can do")
            }
            Self::BadArgument(what) => write!(f, "missing or invalid {what}"),
        }
    }
}

fn number(data: &Value, key: &'static str) -> Result<f64, Error> {
    data.get(key)
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite())
        .ok_or(Error::BadArgument(key))
}

fn text(data: &Value, key: &'static str) -> Result<String, Error> {
    data.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or(Error::BadArgument(key))
}

fn call(domain: &str, service: &str, entity_id: &str, data: Value) -> ServiceCall {
    ServiceCall {
        domain: domain.to_string(),
        service: service.to_string(),
        entity_id: entity_id.to_string(),
        data,
    }
}

pub fn resolve(entity_id: &str, action: &str, data: &Value) -> Result<ServiceCall, Error> {
    if !valid_entity_id(entity_id) {
        return Err(Error::BadEntityId);
    }
    let domain = domain_of(entity_id);
    let unsupported = || Error::Unsupported {
        action: action.to_string(),
        domain: domain.to_string(),
    };
    let none = || Value::Object(Map::new());

    let resolved = match (domain, action) {
        ("light" | "switch" | "fan" | "input_boolean" | "humidifier" | "siren" | "remote"
            | "media_player" | "climate", "toggle") => call(domain, "toggle", entity_id, none()),
        ("light" | "switch" | "fan" | "input_boolean" | "humidifier" | "siren" | "remote"
            | "media_player" | "climate", "turnOn") => call(domain, "turn_on", entity_id, none()),
        ("light" | "switch" | "fan" | "input_boolean" | "humidifier" | "siren" | "remote"
            | "media_player" | "climate", "turnOff") => call(domain, "turn_off", entity_id, none()),

        ("scene", "activate") => call("scene", "turn_on", entity_id, none()),
        ("script", "activate") => call("script", "turn_on", entity_id, none()),
        ("button" | "input_button", "activate") => call(domain, "press", entity_id, none()),

        ("lock", "lock") => call("lock", "lock", entity_id, none()),
        ("lock", "unlock") => call("lock", "unlock", entity_id, none()),

        ("light", "setBrightness") => {
            let value = number(data, "brightness")?;
            if !(0.0..=255.0).contains(&value) {
                return Err(Error::BadArgument("brightness"));
            }
            // Home Assistant rejects brightness 0; the intent is off.
            if value < 1.0 {
                call("light", "turn_off", entity_id, none())
            } else {
                call("light", "turn_on", entity_id, json!({ "brightness": value.round() as u64 }))
            }
        }
        ("light", "setColorTemp") => {
            let kelvin = number(data, "kelvin")?;
            if !(1000.0..=10000.0).contains(&kelvin) {
                return Err(Error::BadArgument("kelvin"));
            }
            call("light", "turn_on", entity_id, json!({ "color_temp_kelvin": kelvin.round() as u64 }))
        }
        ("light", "setColor") => {
            let hue = number(data, "hue")?;
            let saturation = number(data, "saturation")?;
            if !(0.0..=360.0).contains(&hue) || !(0.0..=100.0).contains(&saturation) {
                return Err(Error::BadArgument("hue/saturation"));
            }
            call("light", "turn_on", entity_id, json!({ "hs_color": [hue, saturation] }))
        }

        ("cover", "open") => call("cover", "open_cover", entity_id, none()),
        ("cover", "close") => call("cover", "close_cover", entity_id, none()),
        ("cover", "stop") => call("cover", "stop_cover", entity_id, none()),
        ("cover", "setPosition") => {
            let position = number(data, "position")?;
            if !(0.0..=100.0).contains(&position) {
                return Err(Error::BadArgument("position"));
            }
            call("cover", "set_cover_position", entity_id, json!({ "position": position.round() as u64 }))
        }

        ("media_player", "playPause") => call("media_player", "media_play_pause", entity_id, none()),
        ("media_player", "next") => call("media_player", "media_next_track", entity_id, none()),
        ("media_player", "previous") => call("media_player", "media_previous_track", entity_id, none()),
        ("media_player", "setVolume") => {
            let level = number(data, "level")?;
            if !(0.0..=1.0).contains(&level) {
                return Err(Error::BadArgument("level"));
            }
            call("media_player", "volume_set", entity_id, json!({ "volume_level": level }))
        }
        ("media_player", "setMuted") => {
            let muted = data.get("muted").and_then(Value::as_bool).ok_or(Error::BadArgument("muted"))?;
            call("media_player", "volume_mute", entity_id, json!({ "is_volume_muted": muted }))
        }

        ("fan", "setSpeed") => {
            let percentage = number(data, "percentage")?;
            if !(0.0..=100.0).contains(&percentage) {
                return Err(Error::BadArgument("percentage"));
            }
            call("fan", "set_percentage", entity_id, json!({ "percentage": percentage.round() as u64 }))
        }

        ("climate", "setHvacMode") => {
            call("climate", "set_hvac_mode", entity_id, json!({ "hvac_mode": text(data, "mode")? }))
        }
        ("climate", "setTemperature") => {
            let temperature = number(data, "temperature")?;
            if !TEMPERATURE_RANGE.contains(&temperature) {
                return Err(Error::BadArgument("temperature"));
            }
            call("climate", "set_temperature", entity_id, json!({ "temperature": temperature }))
        }
        ("climate", "setTemperatureRange") => {
            let low = number(data, "low")?;
            let high = number(data, "high")?;
            if low > high
                || !TEMPERATURE_RANGE.contains(&low)
                || !TEMPERATURE_RANGE.contains(&high)
            {
                return Err(Error::BadArgument("low/high"));
            }
            call("climate", "set_temperature", entity_id, json!({ "target_temp_low": low, "target_temp_high": high }))
        }
        ("climate", "setFanMode") => {
            call("climate", "set_fan_mode", entity_id, json!({ "fan_mode": text(data, "mode")? }))
        }
        ("climate", "setPresetMode") => {
            call("climate", "set_preset_mode", entity_id, json!({ "preset_mode": text(data, "mode")? }))
        }

        _ => return Err(unsupported()),
    };
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(entity: &str, action: &str, data: Value) -> ServiceCall {
        resolve(entity, action, &data).unwrap_or_else(|e| panic!("{entity}/{action} should resolve: {e}"))
    }

    #[test]
    fn toggling_maps_onto_the_entitys_own_domain() {
        let c = ok("light.desk", "toggle", json!({}));
        assert_eq!((c.domain.as_str(), c.service.as_str()), ("light", "toggle"));
        let c = ok("switch.pump", "turnOff", json!({}));
        assert_eq!((c.domain.as_str(), c.service.as_str()), ("switch", "turn_off"));
    }

    #[test]
    fn scenes_and_scripts_activate_through_turn_on_but_buttons_press() {
        assert_eq!(ok("scene.movie", "activate", json!({})).service, "turn_on");
        assert_eq!(ok("script.bedtime", "activate", json!({})).service, "turn_on");
        assert_eq!(ok("button.doorbell", "activate", json!({})).service, "press");
    }

    #[test]
    fn brightness_is_clamped_and_zero_turns_the_light_off() {
        let c = ok("light.desk", "setBrightness", json!({ "brightness": 128 }));
        assert_eq!(c.service, "turn_on");
        assert_eq!(c.data["brightness"], 128);

        assert_eq!(ok("light.desk", "setBrightness", json!({ "brightness": 0 })).service, "turn_off");

        assert_eq!(
            resolve("light.desk", "setBrightness", &json!({ "brightness": 999 })),
            Err(Error::BadArgument("brightness"))
        );
        assert_eq!(
            resolve("light.desk", "setBrightness", &json!({ "brightness": -5 })),
            Err(Error::BadArgument("brightness"))
        );
    }

    #[test]
    fn out_of_range_values_are_refused_rather_than_forwarded() {
        assert!(resolve("cover.door", "setPosition", &json!({ "position": 150 })).is_err());
        assert!(resolve("media_player.tv", "setVolume", &json!({ "level": 2.0 })).is_err());
        assert!(resolve("light.a", "setColor", &json!({ "hue": 400, "saturation": 50 })).is_err());
        assert!(resolve("light.a", "setColorTemp", &json!({ "kelvin": 50 })).is_err());
        assert!(resolve("fan.a", "setSpeed", &json!({ "percentage": 101 })).is_err());
    }

    #[test]
    fn a_temperature_band_must_be_the_right_way_round() {
        let c = ok("climate.hall", "setTemperatureRange", json!({ "low": 18, "high": 24 }));
        assert_eq!(c.data["target_temp_low"], 18.0);
        assert_eq!(c.data["target_temp_high"], 24.0);
        assert_eq!(
            resolve("climate.hall", "setTemperatureRange", &json!({ "low": 24, "high": 18 })),
            Err(Error::BadArgument("low/high"))
        );
    }

    #[test]
    fn missing_or_wrongly_typed_arguments_are_refused() {
        assert_eq!(
            resolve("light.desk", "setBrightness", &json!({})),
            Err(Error::BadArgument("brightness"))
        );
        assert_eq!(
            resolve("light.desk", "setBrightness", &json!({ "brightness": "bright" })),
            Err(Error::BadArgument("brightness"))
        );
        assert_eq!(
            resolve("climate.hall", "setHvacMode", &json!({ "mode": "" })),
            Err(Error::BadArgument("mode"))
        );
        assert!(resolve("media_player.tv", "setVolume", &json!({ "level": f64::NAN })).is_err());
    }

    #[test]
    fn temperatures_outside_any_plausible_setpoint_are_refused() {
        assert!(resolve("climate.hall", "setTemperature", &json!({ "temperature": 500 })).is_err());
        assert!(resolve("climate.hall", "setTemperature", &json!({ "temperature": -273 })).is_err());
        assert!(resolve("climate.hall", "setTemperature", &json!({ "temperature": 21 })).is_ok());
        assert!(resolve("climate.hall", "setTemperatureRange",
                        &json!({ "low": -400, "high": 20 })).is_err());
    }

    #[test]
    fn an_action_a_domain_does_not_offer_is_refused() {
        assert!(matches!(
            resolve("sensor.temperature", "toggle", &json!({})),
            Err(Error::Unsupported { .. })
        ));
        assert!(matches!(
            resolve("light.desk", "unlock", &json!({})),
            Err(Error::Unsupported { .. })
        ));
        assert!(matches!(
            resolve("cover.door", "playPause", &json!({})),
            Err(Error::Unsupported { .. })
        ));
    }

    #[test]
    fn there_is_no_action_that_names_its_own_service() {
        for attempt in ["call_service", "homeassistant.restart", "shell_command.rm", "turn_on"] {
            assert!(
                resolve("light.desk", attempt, &json!({ "service": "shell_command.rm" })).is_err()
                    || !matches!(attempt, "call_service" | "homeassistant.restart" | "shell_command.rm"),
                "{attempt} must not resolve"
            );
        }
        let c = ok("light.desk", "toggle", json!({ "domain": "shell_command", "service": "rm" }));
        assert_eq!(c.domain, "light");
        assert_eq!(c.service, "toggle");
    }

    #[test]
    fn malformed_entity_ids_are_refused() {
        for bad in ["", "light", "light.", ".desk", "Light.Desk", "light.desk; rm -rf /", "light..desk"] {
            assert_eq!(
                resolve(bad, "toggle", &json!({})),
                Err(Error::BadEntityId),
                "{bad} should be refused"
            );
        }
    }
}
