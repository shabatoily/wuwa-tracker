use std::collections::BTreeMap;
use wuwa_tracker_types::{CharacterSummary, Stats};

/// 캐릭터 기록을 포함하는 배너 type ID입니다.
const CHARACTER_BANNER_TYPES: [i32; 6] = [1, 3, 5, 6, 8, 10];

#[derive(Default)]
struct CharacterTotal {
    summary: CharacterSummary,
    banners: BTreeMap<i32, String>,
}

/// 캐릭터 배너의 최신순 기록을 캐릭터별 요약으로 집계합니다.
///
/// 각 획득 비용은 직전 5성 이후의 뽑기 횟수를 기준으로 계산합니다. 결과는 등급, 획득 횟수,
/// 최근 획득 시각, 이름 순으로 정렬합니다.
pub fn character_summaries(
    stats: &[Stats],
    character_resource_type: &str,
    astrite_per_pull: usize,
) -> Vec<CharacterSummary> {
    if character_resource_type.is_empty() {
        return Vec::new();
    }

    let mut totals: BTreeMap<i32, CharacterTotal> = BTreeMap::new();
    for stat in stats {
        if !CHARACTER_BANNER_TYPES.contains(&stat.gacha_type) {
            continue;
        }

        let mut pity = 0usize;
        for record in stat.records.iter().rev() {
            pity += 1;
            if record.quality_level != 5 {
                continue;
            }

            if record.resource_type == character_resource_type {
                let total = totals.entry(record.resource_id).or_default();
                total.summary.resource_id = record.resource_id;
                total.summary.name = record.name.clone();
                total.summary.quality_level = record.quality_level;
                total.summary.resource_type = record.resource_type.clone();
                total.summary.copies += 1;
                total.summary.spent_astrite += pity * astrite_per_pull;
                total.summary.last_time = record.time.clone();
                total.banners.insert(
                    stat.gacha_type,
                    if stat.gacha_name.is_empty() {
                        stat.gacha_type.to_string()
                    } else {
                        stat.gacha_name.clone()
                    },
                );
            }
            pity = 0;
        }
    }

    let mut summaries: Vec<_> = totals
        .into_values()
        .map(|mut total| {
            total.summary.banner_count = total.banners.len();
            total.summary.banners = total.banners.into_values().collect();
            total.summary
        })
        .collect();
    summaries.sort_by(|left, right| {
        right
            .quality_level
            .cmp(&left.quality_level)
            .then(right.copies.cmp(&left.copies))
            .then(right.last_time.cmp(&left.last_time))
            .then(left.name.cmp(&right.name))
    });
    summaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use wuwa_tracker_types::Record;

    #[test]
    fn character_summaries_count_copies_and_astrite() {
        let stats = vec![Stats {
            gacha_type: 1,
            gacha_name: "Featured Resonator Convene".to_string(),
            records: vec![
                record(100, 5, "Resonator", "Jiyan", "2026-01-04"),
                record(1, 3, "Weapon", "Sword", "2026-01-03"),
                record(2, 3, "Weapon", "Sword", "2026-01-02"),
                record(100, 5, "Resonator", "Jiyan", "2026-01-01"),
            ],
            ..Default::default()
        }];

        let summaries = character_summaries(&stats, "Resonator", 160);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].copies, 2);
        assert_eq!(summaries[0].spent_astrite, 640);
        assert_eq!(
            summaries[0].banners,
            vec!["Featured Resonator Convene".to_string()]
        );
    }

    #[test]
    fn character_summaries_ignore_weapons_and_weapon_banners() {
        let stats = vec![
            Stats {
                gacha_type: 1,
                records: vec![record(200, 5, "Weapon", "Sword", "2026-01-01")],
                ..Default::default()
            },
            Stats {
                gacha_type: 2,
                records: vec![record(100, 5, "Resonator", "Jiyan", "2026-01-01")],
                ..Default::default()
            },
        ];

        assert!(character_summaries(&stats, "Resonator", 160).is_empty());
    }

    fn record(
        resource_id: i32,
        quality_level: i32,
        resource_type: &str,
        name: &str,
        time: &str,
    ) -> Record {
        Record {
            resource_id,
            quality_level,
            resource_type: resource_type.to_string(),
            name: name.to_string(),
            time: time.to_string(),
            ..Default::default()
        }
    }
}
