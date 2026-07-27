use std::collections::HashMap;

use serde::{Deserialize, Serialize, de};
use serde_json::{Value, map};

use super::team::Team;

/// A single hero pick entry, combining pick{n}_id and pick{n}_class.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PickEntry {
    /// Hero numeric ID (pick{n}_id)
    pub hero_id: i32,
    /// Hero internal name (pick{n}_class), e.g. "npc_dota_hero_antimage"
    pub hero_class: String,
}

/// Draft details for a single team.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DraftDetails {
    pub is_home_team: bool,
    /// Picks indexed by pick order (0..4), empty slots filtered out
    pub picks: HashMap<u8, PickEntry>,
    /// Bans indexed by ban order (0..5), empty slots filtered out
    pub bans: HashMap<u8, PickEntry>,
}

impl<'de> Deserialize<'de> for DraftDetails {
    fn deserialize<D>(deserializer: D) -> Result<DraftDetails, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let m = map::Map::<String, Value>::deserialize(deserializer)?;

        let is_home_team = m
            .get("home_team")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut raw_picks: HashMap<u8, (Option<i32>, Option<String>)> = HashMap::new();
        let mut raw_bans: HashMap<u8, (Option<i32>, Option<String>)> = HashMap::new();

        for (key, value) in &m {
            if let Some(rest) = key.strip_prefix("pick") {
                if let Some(idx_str) = rest.strip_suffix("_id") {
                    if let Ok(idx) = idx_str.parse::<u8>() {
                        raw_picks.entry(idx).or_default().0 = value.as_i64().map(|n| n as i32);
                    }
                } else if let Some(idx_str) = rest.strip_suffix("_class") {
                    if let Ok(idx) = idx_str.parse::<u8>() {
                        raw_picks.entry(idx).or_default().1 = value.as_str().map(|s| s.to_owned());
                    }
                }
            } else if let Some(rest) = key.strip_prefix("ban") {
                if let Some(idx_str) = rest.strip_suffix("_id") {
                    if let Ok(idx) = idx_str.parse::<u8>() {
                        raw_bans.entry(idx).or_default().0 = value.as_i64().map(|n| n as i32);
                    }
                } else if let Some(idx_str) = rest.strip_suffix("_class") {
                    if let Ok(idx) = idx_str.parse::<u8>() {
                        raw_bans.entry(idx).or_default().1 = value.as_str().map(|s| s.to_owned());
                    }
                }
            }
        }

        let picks = collect_entries(raw_picks);
        let bans = collect_entries(raw_bans);

        Ok(DraftDetails { is_home_team, picks, bans })
    }
}

/// Collect raw (id, class) pairs into PickEntry map, filtering empty slots (id=0, class="").
fn collect_entries(raw: HashMap<u8, (Option<i32>, Option<String>)>) -> HashMap<u8, PickEntry> {
    let mut out = HashMap::new();
    for (idx, (hero_id_opt, hero_class_opt)) in raw {
        if let (Some(hero_id), Some(hero_class)) = (hero_id_opt, hero_class_opt) {
            if hero_id != 0 && !hero_class.is_empty() {
                out.insert(idx, PickEntry { hero_id, hero_class });
            }
        }
    }
    out
}

/// Full draft state, top-level `draft` field from GSI.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Draft {
    /// Currently active team (2 = Radiant, 3 = Dire)
    pub active_team: i32,
    /// true = pick phase, false = ban phase
    pub pick: bool,
    /// Remaining time for active team (seconds)
    pub active_team_remaining_time: i32,
    /// Radiant bonus time bank (seconds)
    pub radiant_bonus_time: i32,
    /// Dire bonus time bank (seconds)
    pub dire_bonus_time: i32,
    /// Per-team draft details
    pub teams: HashMap<Team, DraftDetails>,
}

impl<'de> Deserialize<'de> for Draft {
    fn deserialize<D>(deserializer: D) -> Result<Draft, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let m = map::Map::<String, Value>::deserialize(deserializer)?;

        let active_team = m
            .get("activeteam")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let pick = m
            .get("pick")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let active_team_remaining_time = m
            .get("activeteam_time_remaining")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let radiant_bonus_time = m
            .get("radiant_bonus_time")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let dire_bonus_time = m
            .get("dire_bonus_time")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let mut teams = HashMap::new();
        for (key, value) in &m {
            if key.starts_with("team") {
                let team = Team::from(key.clone());
                if let Value::Object(_) = value {
                    let details =
                        DraftDetails::deserialize(value.clone()).map_err(de::Error::custom)?;
                    teams.insert(team, details);
                }
            }
        }

        Ok(Draft {
            active_team,
            pick,
            active_team_remaining_time,
            radiant_bonus_time,
            dire_bonus_time,
            teams,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // JSON from Kotlin version's real test data (draft_spectator.json)
    #[test]
    fn test_draft_deserialize_with_bans() {
        let json = r#"{
            "activeteam": 2,
            "pick": true,
            "activeteam_time_remaining": 10,
            "radiant_bonus_time": 118,
            "dire_bonus_time": 117,
            "team2": {
                "home_team": false,
                "pick0_id": 19,
                "pick0_class": "tiny",
                "pick1_id": 0,
                "pick1_class": "",
                "ban0_id": 135,
                "ban0_class": "dawnbreaker",
                "ban1_id": 101,
                "ban1_class": "skywrath_mage"
            },
            "team3": {
                "home_team": true,
                "pick0_id": 136,
                "pick0_class": "marci",
                "pick1_id": 78,
                "pick1_class": "brewmaster",
                "ban0_id": 92,
                "ban0_class": "visage",
                "ban1_id": 22,
                "ban1_class": "zuus"
            }
        }"#;

        let draft: Draft = serde_json::from_str(json).expect("Failed to deserialize Draft");

        assert_eq!(draft.active_team, 2);
        assert!(draft.pick);
        assert_eq!(draft.active_team_remaining_time, 10);
        assert_eq!(draft.radiant_bonus_time, 118);
        assert_eq!(draft.dire_bonus_time, 117);

        // team2 = Radiant
        let radiant = draft.teams.get(&Team::Radiant).expect("No Radiant team");
        assert!(!radiant.is_home_team);
        // pick1 is empty (id=0, class=""), should be filtered out
        assert_eq!(radiant.picks.len(), 1);
        assert_eq!(radiant.picks[&0].hero_id, 19);
        assert_eq!(radiant.picks[&0].hero_class, "tiny");
        assert_eq!(radiant.bans.len(), 2);
        assert_eq!(radiant.bans[&0].hero_id, 135);
        assert_eq!(radiant.bans[&0].hero_class, "dawnbreaker");
        assert_eq!(radiant.bans[&1].hero_id, 101);
        assert_eq!(radiant.bans[&1].hero_class, "skywrath_mage");

        // team3 = Dire
        let dire = draft.teams.get(&Team::Dire).expect("No Dire team");
        assert!(dire.is_home_team);
        assert_eq!(dire.picks.len(), 2);
        assert_eq!(dire.picks[&0].hero_class, "marci");
        assert_eq!(dire.picks[&1].hero_class, "brewmaster");
        assert_eq!(dire.bans.len(), 2);
        assert_eq!(dire.bans[&0].hero_class, "visage");
        assert_eq!(dire.bans[&1].hero_class, "zuus");
    }

    #[test]
    fn test_empty_slots_filtered() {
        let json = r#"{
            "activeteam": 2,
            "pick": true,
            "activeteam_time_remaining": 25,
            "radiant_bonus_time": 130,
            "dire_bonus_time": 130,
            "team2": {
                "home_team": false,
                "pick0_id": 19,
                "pick0_class": "tiny",
                "pick1_id": 0,
                "pick1_class": "",
                "pick2_id": 0,
                "pick2_class": ""
            }
        }"#;

        let draft: Draft = serde_json::from_str(json).expect("Failed to deserialize");
        let radiant = draft.teams.get(&Team::Radiant).expect("No Radiant team");
        assert_eq!(radiant.picks.len(), 1, "Empty slots should be filtered out");
        assert_eq!(radiant.bans.len(), 0);
    }

    #[test]
    fn test_empty_draft_deserialize() {
        let json = r#"{
            "activeteam": 0,
            "pick": true,
            "activeteam_time_remaining": 0,
            "radiant_bonus_time": 0,
            "dire_bonus_time": 0
        }"#;

        let draft: Draft = serde_json::from_str(json).expect("Failed to deserialize empty Draft");
        assert!(draft.teams.is_empty());
    }
}
