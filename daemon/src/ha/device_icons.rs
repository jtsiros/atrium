// Home Assistant picks an entity's icon from its domain, its device class and,
// for some classes, its state. Ported from the frontend's entity-icon tables so
// the panel shows the same mark as the dashboard.

pub fn for_sensor(device_class: Option<&str>) -> Option<&'static str> {
    Some(match device_class? {
        "apparent_power" | "power" | "reactive_power" => "mdi:flash",
        "aqi" => "mdi:air-filter",
        "atmospheric_pressure" => "mdi:thermometer-lines",
        "battery" => "mdi:battery",
        "carbon_dioxide" => "mdi:molecule-co2",
        "carbon_monoxide" => "mdi:molecule-co",
        "current" => "mdi:current-ac",
        "data_rate" => "mdi:transmission-tower",
        "data_size" => "mdi:database",
        "date" => "mdi:calendar",
        "distance" => "mdi:arrow-left-right",
        "duration" => "mdi:progress-clock",
        "energy" | "energy_distance" => "mdi:lightning-bolt",
        "energy_storage" => "mdi:car-battery",
        "frequency" | "voltage" => "mdi:sine-wave",
        "gas" => "mdi:meter-gas",
        "humidity" | "moisture" => "mdi:water-percent",
        "illuminance" => "mdi:brightness-5",
        "irradiance" => "mdi:sun-wireless",
        "monetary" => "mdi:cash",
        "nitrogen_dioxide" | "nitrogen_monoxide" | "nitrous_oxide" | "ozone" | "pm1" | "pm10"
        | "pm25" | "sulphur_dioxide" | "volatile_organic_compounds" => "mdi:molecule",
        "ph" => "mdi:ph",
        "power_factor" => "mdi:angle-acute",
        "precipitation" => "mdi:weather-rainy",
        "precipitation_intensity" => "mdi:weather-pouring",
        "pressure" => "mdi:gauge",
        "signal_strength" => "mdi:wifi",
        "sound_pressure" => "mdi:ear-hearing",
        "speed" => "mdi:speedometer",
        "temperature" => "mdi:thermometer",
        "timestamp" => "mdi:clock",
        "water" => "mdi:water",
        "weight" => "mdi:weight",
        "wind_speed" => "mdi:weather-windy",
        _ => return None,
    })
}

pub fn for_binary_sensor(device_class: Option<&str>, active: bool) -> Option<&'static str> {
    let (on, off) = match device_class? {
        "battery" => ("mdi:battery-outline", "mdi:battery"),
        "battery_charging" => ("mdi:battery-charging", "mdi:battery"),
        "carbon_monoxide" | "gas" | "problem" | "safety" | "smoke" | "tamper" => {
            ("mdi:alert-circle", "mdi:check-circle")
        }
        "cold" => ("mdi:snowflake", "mdi:thermometer"),
        "connectivity" => ("mdi:check-network-outline", "mdi:close-network-outline"),
        "door" => ("mdi:door-open", "mdi:door-closed"),
        "garage_door" => ("mdi:garage-open", "mdi:garage"),
        "heat" => ("mdi:fire", "mdi:thermometer"),
        "light" => ("mdi:brightness-7", "mdi:brightness-5"),
        "lock" => ("mdi:lock-open", "mdi:lock"),
        "moisture" => ("mdi:water", "mdi:water-off"),
        "motion" => ("mdi:motion-sensor", "mdi:motion-sensor-off"),
        "occupancy" | "presence" => ("mdi:home", "mdi:home-outline"),
        "opening" => ("mdi:square-outline", "mdi:square"),
        "plug" | "power" => ("mdi:power-plug", "mdi:power-plug-off"),
        "running" => ("mdi:play", "mdi:stop"),
        "sound" => ("mdi:music-note", "mdi:music-note-off"),
        "update" => ("mdi:package-up", "mdi:package"),
        "vibration" => ("mdi:vibrate", "mdi:crop-portrait"),
        "window" => ("mdi:window-open", "mdi:window-closed"),
        _ => return None,
    };
    Some(if active { on } else { off })
}

pub fn for_switch(device_class: Option<&str>, active: bool) -> Option<&'static str> {
    match device_class? {
        "outlet" => Some(if active { "mdi:power-plug" } else { "mdi:power-plug-off" }),
        "switch" => Some(if active {
            "mdi:toggle-switch-variant"
        } else {
            "mdi:toggle-switch-variant-off"
        }),
        _ => None,
    }
}

pub fn for_cover(device_class: Option<&str>, active: bool) -> Option<&'static str> {
    let (open, closed) = match device_class? {
        "awning" => ("mdi:window-open", "mdi:window-closed"),
        "blind" => ("mdi:blinds-open", "mdi:blinds"),
        "curtain" => ("mdi:curtains", "mdi:curtains-closed"),
        "damper" => ("mdi:checkbox-blank-circle-outline", "mdi:circle-slice-8"),
        "door" => ("mdi:door-open", "mdi:door-closed"),
        "garage" => ("mdi:garage-open", "mdi:garage"),
        "gate" => ("mdi:gate-open", "mdi:gate"),
        "shade" => ("mdi:roller-shade", "mdi:roller-shade-closed"),
        "shutter" => ("mdi:window-shutter-open", "mdi:window-shutter"),
        "window" => ("mdi:window-open", "mdi:window-closed"),
        _ => return None,
    };
    Some(if active { open } else { closed })
}

pub fn for_media_player(device_class: Option<&str>) -> Option<&'static str> {
    Some(match device_class? {
        "receiver" => "mdi:audio-video",
        "speaker" => "mdi:speaker",
        "tv" => "mdi:television",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ha::icons;

    #[test]
    fn the_classes_on_a_real_instance_all_resolve() {
        for class in ["power", "voltage", "current", "energy", "battery", "temperature",
                      "humidity", "distance", "timestamp", "aqi", "pm25"] {
            let icon = for_sensor(Some(class)).unwrap_or_else(|| panic!("{class} unmapped"));
            assert!(icons::glyph(icon).is_some(), "{class} -> {icon} is not in the font");
        }
        for class in ["connectivity", "occupancy", "problem"] {
            for active in [true, false] {
                let icon = for_binary_sensor(Some(class), active).unwrap();
                assert!(icons::glyph(icon).is_some(), "{class} -> {icon} is not in the font");
            }
        }
        for active in [true, false] {
            assert!(icons::glyph(for_switch(Some("outlet"), active).unwrap()).is_some());
            assert!(icons::glyph(for_cover(Some("garage"), active).unwrap()).is_some());
        }
        for class in ["speaker", "receiver"] {
            assert!(icons::glyph(for_media_player(Some(class)).unwrap()).is_some());
        }
    }

    #[test]
    fn every_mapped_icon_exists_in_the_font() {
        let classes = ["apparent_power", "aqi", "atmospheric_pressure", "battery",
            "carbon_dioxide", "carbon_monoxide", "current", "data_rate", "data_size", "date",
            "distance", "duration", "energy", "energy_storage", "frequency", "gas", "humidity",
            "illuminance", "irradiance", "monetary", "nitrogen_dioxide", "ozone", "ph", "pm1",
            "pm10", "pm25", "power", "power_factor", "precipitation", "precipitation_intensity",
            "pressure", "reactive_power", "signal_strength", "sound_pressure", "speed",
            "sulphur_dioxide", "temperature", "timestamp", "voltage", "volatile_organic_compounds",
            "water", "weight", "wind_speed", "moisture"];
        for class in classes {
            if let Some(icon) = for_sensor(Some(class)) {
                assert!(icons::glyph(icon).is_some(), "sensor/{class} -> {icon} missing");
            }
        }
        let binary = ["battery", "battery_charging", "carbon_monoxide", "cold", "connectivity",
            "door", "garage_door", "gas", "heat", "light", "lock", "moisture", "motion",
            "occupancy", "opening", "plug", "power", "presence", "problem", "running", "safety",
            "smoke", "sound", "tamper", "update", "vibration", "window"];
        for class in binary {
            for active in [true, false] {
                if let Some(icon) = for_binary_sensor(Some(class), active) {
                    assert!(icons::glyph(icon).is_some(), "binary_sensor/{class} -> {icon} missing");
                }
            }
        }
        let covers = ["awning", "blind", "curtain", "damper", "door", "garage", "gate", "shade",
                      "shutter", "window"];
        for class in covers {
            for active in [true, false] {
                let icon = for_cover(Some(class), active).unwrap();
                assert!(icons::glyph(icon).is_some(), "cover/{class} -> {icon} missing");
            }
        }
    }

    #[test]
    fn an_unmapped_class_falls_through_to_the_domain_default() {
        assert_eq!(for_sensor(Some("something_new")), None);
        assert_eq!(for_sensor(None), None);
        assert_eq!(for_binary_sensor(None, true), None);
    }
}
