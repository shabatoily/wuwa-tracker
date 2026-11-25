use crate::AppError;
use serde::Deserialize;
use std::{collections::BTreeMap, time::Duration};
use wuwa_tracker_types::{EchoSet, EchoSetEffect};

const API_URL: &str = "https://api-v2.encore.moe/api";

pub async fn fetch_echo_sets(language: &str) -> Result<Vec<EchoSet>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let response = client
        .get(format!("{API_URL}/{language}/echo"))
        .send()
        .await?
        .error_for_status()?
        .json::<EchoList>()
        .await?;
    collect_sets(response)
}

#[derive(Deserialize)]
struct EchoList {
    #[serde(rename = "Echo")]
    echoes: Vec<Echo>,
}

#[derive(Deserialize)]
struct Echo {
    #[serde(rename = "FetterGroups")]
    sets: Vec<Set>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Set {
    id: i32,
    name: String,
    icon: String,
    fetters: Vec<Effect>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Effect {
    id: i32,
    key: u32,
    effect_description: String,
}

fn collect_sets(response: EchoList) -> Result<Vec<EchoSet>, AppError> {
    let mut sets = BTreeMap::new();
    for echo in response.echoes {
        for set in echo.sets {
            if set.id <= 0 || set.name.trim().is_empty() || set.fetters.is_empty() {
                return Err(AppError::InvalidEchoSetData);
            }
            let mut effects = BTreeMap::new();
            for effect in set.fetters {
                if effect.id <= 0 || effect.key == 0 || effect.effect_description.trim().is_empty()
                {
                    return Err(AppError::InvalidEchoSetData);
                }
                let effect = EchoSetEffect {
                    id: effect.id,
                    required_count: effect.key,
                    description: effect.effect_description,
                };
                if effects.insert(effect.required_count, effect).is_some() {
                    return Err(AppError::InvalidEchoSetData);
                }
            }
            let set = EchoSet {
                id: set.id,
                name: set.name,
                icon: set.icon,
                effects: effects.into_values().collect(),
            };
            // 동일 세트가 여러 에코에 반복되므로 ID로 합치되 불일치는 숨기지 않습니다.
            if let Some(previous) = sets.insert(set.id, set.clone()) {
                if previous != set {
                    return Err(AppError::InvalidEchoSetData);
                }
            }
        }
    }
    if sets.is_empty() {
        return Err(AppError::InvalidEchoSetData);
    }
    Ok(sets.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sets_are_deduplicated_sorted_and_preserve_effects() {
        let set = json!({"Id": 2, "Name": "Set", "Icon": "icon.webp", "Fetters": [
            {"Id": 25, "Key": 5, "EffectDescription": "Bonus {0}"},
            {"Id": 22, "Key": 2, "EffectDescription": "Bonus 10%"}
        ]});
        let mut first = set.clone();
        first["Id"] = json!(1);
        let response = serde_json::from_value(json!({"Echo": [
            {"FetterGroups": [set.clone(), first]}, {"FetterGroups": [set]}
        ]}))
        .unwrap();
        let sets = collect_sets(response).unwrap();
        assert_eq!(sets.iter().map(|set| set.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(sets[0].effects[0].required_count, 2);
        assert_eq!(sets[0].effects[1].description, "Bonus {0}");
    }

    #[test]
    fn empty_invalid_and_conflicting_sets_are_rejected() {
        let set = json!({"Id": 1, "Name": "Set", "Icon": "", "Fetters": [
            {"Id": 12, "Key": 2, "EffectDescription": "Bonus"}
        ]});
        let mut conflict = set.clone();
        conflict["Name"] = json!("Other");
        let mut invalid = set.clone();
        invalid["Fetters"][0]["Key"] = json!(0);
        for value in [
            json!({"Echo": []}),
            json!({"Echo": [{"FetterGroups": [invalid]}]}),
            json!({"Echo": [{"FetterGroups": [set, conflict]}]}),
        ] {
            assert!(matches!(
                collect_sets(serde_json::from_value(value).unwrap()),
                Err(AppError::InvalidEchoSetData)
            ));
        }
        assert!(serde_json::from_value::<EchoList>(json!({"phantomsList": []})).is_err());
    }
}
