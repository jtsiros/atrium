use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ha::{filter, icons, text};

pub const DEFAULT_AREA_ICON: &str = "mdi:texture-box";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AreaEntry {
    pub area_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub floor_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeviceEntry {
    pub id: String,
    #[serde(default)]
    pub area_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EntityEntry {
    pub entity_id: String,
    #[serde(default)]
    pub area_id: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub entity_category: Option<String>,
    #[serde(default)]
    pub hidden_by: Option<String>,
    #[serde(default)]
    pub disabled_by: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaPrefs {
    #[serde(default)]
    pub order: Vec<String>,
    #[serde(default)]
    pub hidden: Vec<String>,
    #[serde(default)]
    pub hide_entities_without_area: bool,
    #[serde(default = "default_true")]
    pub hide_empty_areas: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AreaPrefs {
    fn default() -> Self {
        Self {
            order: Vec::new(),
            hidden: Vec::new(),
            hide_entities_without_area: false,
            hide_empty_areas: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub glyph: String,
    pub entity_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaChoice {
    pub area_id: String,
    pub name: String,
    pub icon: String,
    pub glyph: String,
    pub entity_count: usize,
    pub hidden: bool,
}

#[derive(Debug, Default)]
pub struct Registry {
    pub areas: Vec<AreaEntry>,
    pub devices: Vec<DeviceEntry>,
    pub entities: Vec<EntityEntry>,
}

fn label(name: &str, fallback: &str) -> String {
    let cleaned = text::plain(name);
    if cleaned.is_empty() {
        text::plain(fallback)
    } else {
        cleaned
    }
}

fn glyph_for(icon: &str) -> String {
    icons::glyph(icon)
        .or_else(|| icons::glyph(DEFAULT_AREA_ICON))
        .map(String::from)
        .unwrap_or_default()
}

impl Registry {
    fn entity_index(&self) -> HashMap<&str, &EntityEntry> {
        self.entities
            .iter()
            .map(|e| (e.entity_id.as_str(), e))
            .collect()
    }

    fn device_area(&self) -> HashMap<&str, &str> {
        self.devices
            .iter()
            .filter_map(|d| d.area_id.as_deref().map(|a| (d.id.as_str(), a)))
            .collect()
    }

    fn area_for<'a>(
        entry: Option<&'a EntityEntry>,
        device_area: &HashMap<&'a str, &'a str>,
    ) -> Option<&'a str> {
        let entry = entry?;
        if let Some(area) = entry.area_id.as_deref() {
            if !area.is_empty() {
                return Some(area);
            }
        }
        let device_id = entry.device_id.as_deref()?;
        device_area.get(device_id).copied()
    }

    fn bucket<'a>(&'a self, live_ids: &'a [String]) -> (BTreeMap<&'a str, Vec<&'a str>>, Vec<&'a str>) {
        let entities = self.entity_index();
        let device_area = self.device_area();

        let mut by_area: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut arealess: Vec<&str> = Vec::new();

        for id in live_ids {
            let entry = entities.get(id.as_str()).copied();
            if !filter::is_visible(id, entry) {
                continue;
            }
            match Self::area_for(entry, &device_area) {
                Some(area) => by_area.entry(area).or_default().push(id.as_str()),
                None => arealess.push(id.as_str()),
            }
        }
        (by_area, arealess)
    }


    fn sort_areas(&self, prefs: &AreaPrefs, ids: &mut Vec<&str>) {
        let rank: HashMap<&str, usize> = prefs
            .order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        let name_of: HashMap<&str, &str> = self
            .areas
            .iter()
            .map(|a| (a.area_id.as_str(), a.name.as_str()))
            .collect();
        let last = prefs.order.len();
        ids.sort_by(|a, b| {
            let ra = rank.get(a).copied().unwrap_or(last);
            let rb = rank.get(b).copied().unwrap_or(last);
            ra.cmp(&rb).then_with(|| {
                let na = name_of.get(a).copied().unwrap_or(a).to_lowercase();
                let nb = name_of.get(b).copied().unwrap_or(b).to_lowercase();
                na.cmp(&nb)
            })
        });
    }

    fn sort_entities(&self, ids: &mut [&str], display_name: impl Fn(&str) -> String) {
        ids.sort_by_cached_key(|id| display_name(id).to_lowercase());
    }

    pub fn project_tabs(
        &self,
        live_ids: &[String],
        prefs: &AreaPrefs,
        display_name: impl Fn(&str) -> String,
    ) -> Vec<Tab> {
        let (by_area, arealess) = self.bucket(live_ids);
        let hidden: HashSet<&str> = prefs.hidden.iter().map(String::as_str).collect();

        let mut area_ids: Vec<&str> = self
            .areas
            .iter()
            .map(|a| a.area_id.as_str())
            .filter(|id| !hidden.contains(id))
            .filter(|id| !prefs.hide_empty_areas || by_area.contains_key(id))
            .collect();
        self.sort_areas(prefs, &mut area_ids);

        let mut tabs: Vec<Tab> = Vec::with_capacity(area_ids.len() + 1);
        for area_id in area_ids {
            let Some(area) = self.areas.iter().find(|a| a.area_id == area_id) else {
                continue;
            };
            let mut ids: Vec<&str> = by_area.get(area_id).cloned().unwrap_or_default();
            self.sort_entities(&mut ids, &display_name);
            let icon = area
                .icon
                .clone()
                .filter(|i| !i.is_empty())
                .unwrap_or_else(|| DEFAULT_AREA_ICON.to_string());
            tabs.push(Tab {
                id: format!("area:{area_id}"),
                title: label(&area.name, area_id),
                glyph: glyph_for(&icon),
                icon,
                entity_ids: ids.into_iter().map(str::to_string).collect(),
            });
        }

        if !prefs.hide_entities_without_area && !arealess.is_empty() {
            let mut ids = arealess;
            self.sort_entities(&mut ids, &display_name);
            tabs.push(Tab {
                id: "unassigned".into(),
                title: "Unassigned".into(),
                glyph: glyph_for("mdi:help-circle-outline"),
                icon: "mdi:help-circle-outline".into(),
                entity_ids: ids.into_iter().map(str::to_string).collect(),
            });
        }
        tabs
    }

    pub fn area_choices(&self, live_ids: &[String], prefs: &AreaPrefs) -> Vec<AreaChoice> {
        let (by_area, _) = self.bucket(live_ids);
        let hidden: HashSet<&str> = prefs.hidden.iter().map(String::as_str).collect();

        let mut ids: Vec<&str> = self.areas.iter().map(|a| a.area_id.as_str()).collect();
        self.sort_areas(prefs, &mut ids);

        ids.iter()
            .filter_map(|area_id| {
                let area = self.areas.iter().find(|a| a.area_id == *area_id)?;
                let icon = area
                    .icon
                    .clone()
                    .filter(|i| !i.is_empty())
                    .unwrap_or_else(|| DEFAULT_AREA_ICON.to_string());
                Some(AreaChoice {
                    area_id: area.area_id.clone(),
                    name: label(&area.name, &area.area_id),
                    glyph: glyph_for(&icon),
                    icon,
                        entity_count: by_area.get(area_id).map_or(0, Vec::len),
                    hidden: hidden.contains(area_id),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(id: &str, name: &str, icon: Option<&str>) -> AreaEntry {
        AreaEntry {
            area_id: id.into(),
            name: name.into(),
            icon: icon.map(str::to_string),
            floor_id: None,
        }
    }

    fn entity(id: &str, area: Option<&str>, device: Option<&str>) -> EntityEntry {
        EntityEntry {
            entity_id: id.into(),
            area_id: area.map(str::to_string),
            device_id: device.map(str::to_string),
            ..Default::default()
        }
    }

    fn fixture() -> Registry {
        Registry {
            areas: vec![
                area("office", "Office", None),
                area("kitchen", "Kitchen", Some("mdi:stove")),
                area("living_room", "Living Room", Some("mdi:sofa")),
                area("garage", "Garage", None),
                area("guest_room", "Guest Room", None),
            ],
            devices: vec![DeviceEntry {
                id: "dev1".into(),
                area_id: Some("living_room".into()),
            }],
            entities: vec![
                entity("light.desk", Some("office"), None),
                entity("switch.fan", Some("office"), None),
                entity("light.counter", Some("kitchen"), None),
                entity("media_player.tv", None, Some("dev1")),
                entity("cover.door", Some("garage"), None),
                entity("sensor.orphan", None, None),
            ],
        }
    }

    fn live(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    fn all_live() -> Vec<String> {
        live(&[
            "light.desk",
            "switch.fan",
            "light.counter",
            "media_player.tv",
            "cover.door",
            "sensor.orphan",
        ])
    }

    fn ident(id: &str) -> String {
        id.to_string()
    }

    fn titles(tabs: &[Tab]) -> Vec<&str> {
        tabs.iter().map(|t| t.title.as_str()).collect()
    }

    fn area_titles(tabs: &[Tab]) -> Vec<&str> {
        tabs.iter()
            .filter(|t| t.id.starts_with("area:"))
            .map(|t| t.title.as_str())
            .collect()
    }

    #[test]
    fn defaults_show_areas_and_keep_unassigned_things_reachable() {
        let prefs = AreaPrefs::default();
        assert!(prefs.order.is_empty());
        assert!(prefs.hidden.is_empty());
        assert!(!prefs.hide_entities_without_area, "do not silently drop entities");
        assert!(prefs.hide_empty_areas, "an empty area is noise as a bar tab");
    }

    #[test]
    fn areas_appear_with_no_favorites_configured() {
        let reg = fixture();
        let tabs = reg.project_tabs(&all_live(), &AreaPrefs::default(), ident);
        assert_eq!(area_titles(&tabs), ["Garage", "Kitchen", "Living Room", "Office"]);
    }

    #[test]
    fn device_area_is_inherited_by_its_entities() {
        let reg = fixture();
        let tabs = reg.project_tabs(&all_live(), &AreaPrefs::default(), ident);
        let living = tabs.iter().find(|t| t.title == "Living Room").unwrap();
        assert_eq!(living.entity_ids, ["media_player.tv"]);
    }

    #[test]
    fn empty_areas_are_hidden_by_default_and_shown_on_request() {
        let reg = fixture();
        let tabs = reg.project_tabs(&all_live(), &AreaPrefs::default(), ident);
        assert!(!area_titles(&tabs).contains(&"Guest Room"));

        let prefs = AreaPrefs {
            hide_empty_areas: false,
            ..Default::default()
        };
        let tabs = reg.project_tabs(&all_live(), &prefs, ident);
        let guest = tabs.iter().find(|t| t.title == "Guest Room").unwrap();
        assert!(guest.entity_ids.is_empty());
    }

    #[test]
    fn hidden_areas_are_dropped_and_order_is_honored() {
        let reg = fixture();
        let prefs = AreaPrefs {
            order: vec!["living_room".into(), "office".into()],
            hidden: vec!["kitchen".into()],
            ..Default::default()
        };
        let tabs = reg.project_tabs(&all_live(), &prefs, ident);
        assert_eq!(area_titles(&tabs), ["Living Room", "Office", "Garage"]);
    }

    #[test]
    fn stale_ids_in_prefs_are_ignored() {
        let reg = fixture();
        let prefs = AreaPrefs {
            order: vec!["ghost_area".into(), "office".into()],
            hidden: vec!["also_gone".into()],
            ..Default::default()
        };
        let tabs = reg.project_tabs(&all_live(), &prefs, ident);
        assert_eq!(area_titles(&tabs), ["Office", "Garage", "Kitchen", "Living Room"]);
    }

    #[test]
    fn arealess_entities_bucket_into_a_trailing_tab() {
        let reg = fixture();
        let hide = AreaPrefs {
            hide_entities_without_area: true,
            ..Default::default()
        };
        assert!(!titles(&reg.project_tabs(&all_live(), &hide, ident)).contains(&"Unassigned"));

        let tabs = reg.project_tabs(&all_live(), &AreaPrefs::default(), ident);
        let unassigned = tabs.iter().find(|t| t.title == "Unassigned").unwrap();
        assert_eq!(unassigned.entity_ids, ["sensor.orphan"]);
        assert_eq!(tabs.last().unwrap().title, "Unassigned");
    }

    #[test]
    fn invisible_entities_never_reach_a_tab() {
        let mut reg = fixture();
        reg.entities.push(EntityEntry {
            entity_id: "sensor.uptime".into(),
            area_id: Some("garage".into()),
            entity_category: Some("diagnostic".into()),
            ..Default::default()
        });
        let mut ids = all_live();
        ids.push("sensor.uptime".into());
        ids.push("automation.nightly".into());

        let tabs = reg.project_tabs(&ids, &AreaPrefs::default(), ident);
        let garage = tabs.iter().find(|t| t.title == "Garage").unwrap();
        assert_eq!(garage.entity_ids, ["cover.door"]);
        assert!(!tabs.iter().any(|t| t.entity_ids.iter().any(|e| e == "automation.nightly")));
    }

    #[test]
    fn registry_entries_without_state_are_not_drawn() {
        let reg = fixture();
        let ids = live(&["light.desk", "switch.fan"]);
        let tabs = reg.project_tabs(&ids, &AreaPrefs::default(), ident);
        assert_eq!(area_titles(&tabs), ["Office"]);
    }

    #[test]
    fn entities_sort_by_display_name_not_entity_id() {
        let reg = fixture();
        let tabs = reg.project_tabs(&all_live(), &AreaPrefs::default(), |id| match id {
            "light.desk" => "Zebra lamp".into(),
            "switch.fan" => "Apple fan".into(),
            other => other.to_string(),
        });
        let office = tabs.iter().find(|t| t.title == "Office").unwrap();
        assert_eq!(office.entity_ids, ["switch.fan", "light.desk"]);
    }

    #[test]
    fn areas_without_an_icon_fall_back_to_the_generic_one() {
        let reg = fixture();
        let tabs = reg.project_tabs(&all_live(), &AreaPrefs::default(), ident);
        let garage = tabs.iter().find(|t| t.title == "Garage").unwrap();
        assert_eq!(garage.icon, DEFAULT_AREA_ICON);
        let kitchen = tabs.iter().find(|t| t.title == "Kitchen").unwrap();
        assert_eq!(kitchen.icon, "mdi:stove");
    }

    #[test]
    fn picker_lists_every_area_including_hidden_and_empty_ones() {
        let reg = fixture();
        let prefs = AreaPrefs {
            hidden: vec!["kitchen".into()],
            ..Default::default()
        };
        let choices = reg.area_choices(&all_live(), &prefs);
        assert_eq!(choices.len(), 5, "every area must be togglable");

        let kitchen = choices.iter().find(|c| c.area_id == "kitchen").unwrap();
        assert!(kitchen.hidden);
        assert_eq!(kitchen.entity_count, 1, "count survives being hidden");

        let guest = choices.iter().find(|c| c.area_id == "guest_room").unwrap();
        assert!(!guest.hidden);
        assert_eq!(guest.entity_count, 0);
    }

    #[test]
    fn the_picker_follows_the_stored_order() {
        let reg = fixture();
        let prefs = AreaPrefs {
            order: vec!["garage".into(), "office".into()],
            ..Default::default()
        };
        let choices = reg.area_choices(&all_live(), &prefs);
        let names: Vec<&str> = choices.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Garage", "Office", "Guest Room", "Kitchen", "Living Room"]);
    }

    #[test]
    fn wire_names_the_panel_reads_are_camel_case() {
        let reg = fixture();
        let tabs = reg.project_tabs(&all_live(), &AreaPrefs::default(), ident);
        let json = serde_json::to_value(&tabs[0]).unwrap();
        for key in ["id", "title", "icon", "glyph", "entityIds"] {
            assert!(json.get(key).is_some(), "Tab is missing {key}: {json}");
        }

        let choices = reg.area_choices(&all_live(), &AreaPrefs::default());
        let json = serde_json::to_value(&choices[0]).unwrap();
        for key in ["areaId", "name", "icon", "glyph", "entityCount", "hidden"] {
            assert!(json.get(key).is_some(), "AreaChoice is missing {key}: {json}");
        }
    }

}
