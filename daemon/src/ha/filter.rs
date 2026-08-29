use crate::ha::registry::EntityEntry;

// HIDE_DOMAIN, HIDE_PLATFORM and the entity_category rule are ported from Home
// Assistant 2026.8 generate-lovelace-config.ts, so that the panel shows what the
// Overview dashboard shows.
const HIDE_DOMAIN: &[&str] = &[
    "ai_task",
    "assist_satellite",
    "automation",
    "configurator",
    "device_tracker",
    "event",
    "geo_location",
    "notify",
    "persistent_notification",
    "script",
    "sun",
    "tag",
    "todo",
    "zone",
];

const HIDE_PLATFORM: &[&str] = &["backup", "mobile_app"];

pub fn domain_of(entity_id: &str) -> &str {
    match entity_id.find('.') {
        Some(i) => &entity_id[..i],
        None => entity_id,
    }
}

pub fn is_hidden_domain(entity_id: &str) -> bool {
    HIDE_DOMAIN.contains(&domain_of(entity_id))
}

pub fn is_visible(entity_id: &str, entry: Option<&EntityEntry>) -> bool {
    if is_hidden_domain(entity_id) {
        return false;
    }
    let Some(entry) = entry else {
        return true;
    };
    if entry.disabled_by.is_some() || entry.hidden_by.is_some() {
        return false;
    }
    if entry.entity_category.is_some() {
        return false;
    }
    if let Some(platform) = entry.platform.as_deref() {
        if HIDE_PLATFORM.contains(&platform) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ha::registry::EntityEntry;

    fn entry() -> EntityEntry {
        EntityEntry {
            entity_id: "light.desk".into(),
            ..Default::default()
        }
    }

    #[test]
    fn domain_split_handles_missing_dot() {
        assert_eq!(domain_of("light.desk"), "light");
        assert_eq!(domain_of("bogus"), "bogus");
    }

    #[test]
    fn plain_entity_is_visible() {
        assert!(is_visible("light.desk", Some(&entry())));
    }

    #[test]
    fn unregistered_entity_is_visible() {
        assert!(is_visible("light.desk", None));
    }

    #[test]
    fn hidden_domains_never_show_even_unregistered() {
        for id in ["automation.morning", "script.bed", "sun.sun", "zone.home"] {
            assert!(!is_visible(id, None), "{id} should be hidden");
        }
    }

    #[test]
    fn category_disabled_and_hidden_entities_are_dropped() {
        let mut e = entry();
        e.entity_category = Some("config".into());
        assert!(!is_visible("light.desk", Some(&e)));

        let mut e = entry();
        e.entity_category = Some("diagnostic".into());
        assert!(!is_visible("light.desk", Some(&e)));

        let mut e = entry();
        e.disabled_by = Some("integration".into());
        assert!(!is_visible("light.desk", Some(&e)));

        let mut e = entry();
        e.hidden_by = Some("user".into());
        assert!(!is_visible("light.desk", Some(&e)));
    }

    #[test]
    fn bookkeeping_platforms_are_dropped() {
        let mut e = entry();
        e.platform = Some("mobile_app".into());
        assert!(!is_visible("sensor.phone_battery", Some(&e)));
    }
}
