use crate::config::Config;
use wuwa_tracker_types::{FiveStarRecord, GachaType, Record, Stats};

#[derive(Debug, Clone)]
/// 배너 규칙과 재화 설정을 적용해 뽑기 통계를 계산합니다.
pub struct StatsCalculator {
    standard_five_star_resources: Vec<i32>,
    astrite_per_pull: usize,
}

impl StatsCalculator {
    pub fn new(config: &Config) -> Self {
        Self {
            standard_five_star_resources: config.standard_five_star_resources.clone(),
            astrite_per_pull: config.astrite_per_pull,
        }
    }

    /// 최신순으로 정렬된 기록에서 pity, 획득률과 운 점수를 계산합니다.
    ///
    /// 반환되는 원본 기록과 5성 목록도 최신순을 유지합니다. 운 점수는 픽업 획득까지 이어진
    /// 빗나감과 확정 획득을 하나의 주기로 계산합니다.
    pub fn calc(&self, records: &[Record], gacha_type: &GachaType) -> Stats {
        let mut stats = Stats {
            gacha_type: gacha_type.id,
            gacha_name: gacha_type.name.clone(),
            total_pulls: records.len(),
            total_astrite: records.len() * self.astrite_per_pull,
            current_pity5: 0,
            current_pity4: 0,
            base_rate: gacha_type.base_rate,
            expected_pulls: gacha_type.expected_pulls,
            five_stars: Vec::new(),
            records: Vec::new(),
            avg_pulls: 0.0,
            actual_rate: 0.0,
            luck_score: 0.0,
            has_five_star: false,
        };

        let mut pity5 = 0;
        let mut pity4 = 0;

        for record in records.iter().rev() {
            pity5 += 1;
            pity4 += 1;
            stats.records.push(record.clone());

            match record.quality_level {
                5 => {
                    // 빗나감이 없는 배너에서는 모든 5성을 픽업 획득으로 취급합니다.
                    let is_pick_up = !gacha_type.has_off_banner_drop
                        || !self
                            .standard_five_star_resources
                            .contains(&record.resource_id);
                    stats.five_stars.push(FiveStarRecord {
                        name: record.name.clone(),
                        time: record.time.clone(),
                        pity: pity5,
                        is_pick_up,
                    });
                    pity5 = 0;
                }
                4 => pity4 = 0,
                _ => {}
            }
        }

        stats.current_pity5 = pity5;
        stats.current_pity4 = pity4;

        let five_star_count = stats.five_stars.len();
        if five_star_count > 0 {
            stats.has_five_star = true;
            if stats.total_pulls > 0 {
                stats.actual_rate = five_star_count as f64 / stats.total_pulls as f64 * 100.0;
            }

            let mut pickup_count = 0;
            let mut actual_total = 0;
            let mut current_cycle_pulls = 0;

            for five_star in &stats.five_stars {
                // 픽뚫 비용은 다음 픽업 획득까지 누적해 하나의 완료 주기로 평가합니다.
                current_cycle_pulls += five_star.pity;
                if five_star.is_pick_up {
                    pickup_count += 1;
                    actual_total += current_cycle_pulls;
                    current_cycle_pulls = 0;
                }
            }

            // 미완료 픽뚫과 현재 pity는 제외하여 평균과 운 점수의 평가 대상을 일치시킵니다.
            if actual_total > 0 {
                stats.avg_pulls = actual_total as f64 / pickup_count as f64;
                stats.luck_score = gacha_type.expected_pulls as f64 / stats.avg_pulls * 100.0;
            }
        }

        stats.five_stars.reverse();
        stats.records.reverse();
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records(cycles: &[(usize, Option<i32>)]) -> Vec<Record> {
        let mut records = Vec::new();
        for &(pulls, resource_id) in cycles {
            records.extend((0..pulls).map(|index| Record {
                quality_level: if index + 1 == pulls && resource_id.is_some() {
                    5
                } else {
                    3
                },
                resource_id: resource_id.unwrap_or_default(),
                ..Default::default()
            }));
        }
        records.reverse();
        records
    }

    #[test]
    fn pickup_efficiency_uses_completed_cycles_only() {
        let config = Config::default();
        let calculator = StatsCalculator::new(&config);
        let banner = &config.gacha_types[0];
        let completed = calculator.calc(&records(&[(60, Some(1203)), (70, Some(9999))]), banner);
        assert_eq!(completed.avg_pulls, 130.0);
        assert!((completed.luck_score - 80.0 / 130.0 * 100.0).abs() < 1e-10);
        assert!(completed.has_pick_up());
        let unfinished = calculator.calc(
            &records(&[
                (60, Some(1203)),
                (70, Some(9999)),
                (20, Some(1203)),
                (10, None),
            ]),
            banner,
        );
        assert_eq!(unfinished.avg_pulls, completed.avg_pulls);
        assert_eq!(unfinished.luck_score, completed.luck_score);
        assert_eq!(unfinished.current_pity5, 10);
        assert_eq!(unfinished.five_stars.len(), 3);
        assert!((unfinished.actual_rate - 3.0 / 160.0 * 100.0).abs() < 1e-10);
        for cycles in [vec![], vec![(10, None)], vec![(60, Some(1203)), (10, None)]] {
            let stats = calculator.calc(&records(&cycles), banner);
            assert!(!stats.has_pick_up());
            assert_eq!(stats.avg_pulls, 0.0);
            assert_eq!(stats.luck_score, 0.0);
        }
        let multiple = calculator.calc(
            &records(&[(60, Some(1203)), (70, Some(9999)), (30, Some(9999))]),
            banner,
        );
        assert_eq!(multiple.avg_pulls, 80.0);
        assert_eq!(multiple.luck_score, 100.0);
        let guaranteed = calculator.calc(
            &records(&[(60, Some(1203)), (70, Some(9999)), (10, None)]),
            &config.gacha_types[1],
        );
        assert_eq!(guaranteed.avg_pulls, 65.0);
        assert!((guaranteed.luck_score - 55.0 / 65.0 * 100.0).abs() < 1e-10);
    }
}
