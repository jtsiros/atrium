const MAX_LEN: usize = 128;

// Home Assistant strings reach QML components this plugin does not own, and the
// shell's shared Dropdown renders its labels as Text.AutoText. Stripping the
// angle brackets is what stops a renamed area from being parsed as markup
// there; every Text element in this repository pins PlainText already.
pub fn plain(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|c| !c.is_control() && *c != '<' && *c != '>')
        .collect();
    let trimmed = cleaned.trim();
    match trimmed.char_indices().nth(MAX_LEN) {
        Some((at, _)) => trimmed[..at].trim_end().to_string(),
        None => trimmed.to_string(),
    }
}

pub fn valid_entity_id(value: &str) -> bool {
    let Some((domain, object)) = value.split_once('.') else {
        return false;
    };
    let ok = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    ok(domain) && ok(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_cannot_survive_into_a_label() {
        assert_eq!(plain("<img src=x onerror=alert(1)>"), "img src=x onerror=alert(1)");
        assert_eq!(plain("<b>Kitchen</b>"), "bKitchen/b");
        assert_eq!(plain("Kitchen"), "Kitchen");
    }

    #[test]
    fn control_characters_cannot_break_the_line_protocol() {
        assert_eq!(plain("Liv\ning\troom"), "Livingroom");
        assert_eq!(plain("Office\u{0}"), "Office");
    }

    #[test]
    fn ordinary_names_including_punctuation_are_untouched() {
        assert_eq!(plain("Bed & Breakfast"), "Bed & Breakfast");
        assert_eq!(plain("Kid's Room"), "Kid's Room");
        assert_eq!(plain("Küche · 2F"), "Küche · 2F");
    }

    #[test]
    fn absurd_names_are_capped_without_splitting_a_character() {
        let long = "é".repeat(500);
        let out = plain(&long);
        assert!(out.chars().count() <= MAX_LEN);
        assert!(out.starts_with('é'));
    }

    #[test]
    fn entity_ids_must_be_a_plain_domain_and_object() {
        assert!(valid_entity_id("light.desk"));
        assert!(valid_entity_id("binary_sensor.front_door_2"));
        for bad in ["", "light", "light.", ".desk", "Light.Desk", "__proto__",
                    "light.desk; rm -rf /", "constructor", "toString", "light..desk"] {
            assert!(!valid_entity_id(bad), "{bad} must be refused");
        }
    }
}
