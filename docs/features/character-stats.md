# Character Stats Page Plan

- 작성일: 2026-07-09
- 수정일: 2026-09-30 (픽업 획득 평균 및 운 점수 요구사항 추가)
- 대상: `crates/wuwa-tracker-webui`
- 상태: planning

## Summary

현재 WebUI 대시보드는 배너별 뽑기 히스토리와 pity 중심의 요약을 보여준다. 이번 기능은 같은 저장 데이터를 캐릭터 기준으로 다시 묶어, 사용자가 보유 캐릭터 풀과 특정 캐릭터에 사용한 재화를 빠르게 확인할 수 있는 별도 페이지를 추가하는 것이다.

1차 구현은 새 저장소 스키마나 전용 API를 만들지 않는다. 이미 `StatsResponse.stats[].records`에 원본 기록이 포함되어 있으므로 WebUI에서 파생 통계를 계산한다. 캐릭터/무기 구분에 필요한 locale 값만 기존 config 응답에 포함한다. 나중에 CLI 리포트나 export에서도 같은 통계가 필요해질 때만 core 계층으로 이동한다.

## Goals

- 뽑기 히스토리에 등장한 캐릭터 목록을 별도 화면에서 제공한다.
- 캐릭터를 선택하면 해당 캐릭터의 획득 횟수와 총 소모 별의소리를 보여준다.
- 기존 대시보드와 섞지 않고 WebUI 안에 새 페이지 흐름을 추가한다.
- 현재 player 선택, scan, track, upload로 갱신되는 `StatsResponse`를 그대로 사용한다.

## Non-goals

- 게임 전체 캐릭터 도감 제공
- 캐릭터 이미지, 속성, 무기 타입 같은 외부 메타데이터 추가
- 전용 백엔드 API, store JSON 구조, export 포맷 변경
- 상세 획득 히스토리 테이블 재구현
- CLI에서 WebUI 수준의 상세 화면 제공

## Current Evidence

- 공용 타입은 `crates/wuwa-tracker-types/src/lib.rs`의 `StatsResponse`, `Stats`, `Record`를 사용한다.
- `Stats.records`는 WebUI에 이미 전달되며, `Record`에는 `resource_id`, `quality_level`, `resource_type`, `name`, `time`이 있다.
- 현재 WebUI는 `crates/wuwa-tracker-webui/src/app.rs`에서 단일 `AppState`와 컴포넌트 함수들로 구성되어 있고 별도 router는 없다.
- 배너별 통계 계산은 `crates/wuwa-tracker-core/src/stats.rs`의 `StatsCalculator`가 담당한다.
- 1회 뽑기 재화는 `crates/wuwa-tracker-core/src/config.rs`에서 `astrite_per_pull = 160`으로 고정되어 있으며, 기존 `Stats.total_astrite`도 같은 값을 사용한다.
- `ConfigResponse.resource_types.character`는 현재 game locale의 캐릭터 타입 이름을 WebUI에 제공한다.

## Terms

- 캐릭터: `Record.resource_type`이 현재 game locale의 `character` 값과 같은 기록. 한국어는 `공명자`, 영어는 `Resonator`가 된다.
- 총 소모 별의소리: 해당 캐릭터를 얻기까지 누적된 뽑기 횟수에 160을 곱한 값.

## Data Model

WebUI 내부 파생 타입으로 충분하다.

```rust
struct CharacterSummary {
    resource_id: i32,
    name: String,
    quality_level: i32,
    resource_type: String,
    copies: usize,
    spent_astrite: usize,
    banner_count: usize,
    banners: Vec<String>,
    last_time: String,
}
```

상세 화면도 동일 타입을 우선 사용한다. 필요한 경우에만 `Vec<CharacterObtain>` 같은 획득 목록을 내부 계산에 보관하되, UI에는 상세 히스토리를 노출하지 않는다.

## Aggregation Rule

1. 활성 player의 모든 `Stats.records`를 배너 단위로 순회한다.
2. 캐릭터 배너만 대상으로 삼는다. 1차 후보는 `Stats.gacha_type` 기준 `1`, `3`, `5`, `6`, `8`, `10`이다.
3. 각 배너의 기록은 `Stats.records.iter().rev()`로 오래된 기록부터 최신 기록까지 순회한다.
4. 배너마다 pity counter를 0부터 증가시킨다.
5. `Record.quality_level == 5`이고 `Record.resource_type`이 현재 game locale의 `character` 값과 같은 기록을 만나면 해당 캐릭터의 `copies`를 1 증가시키고, 이번 획득 비용으로 `pity * 160`을 더한다.
6. 5성 기록을 만난 뒤 pity counter를 0으로 초기화한다.

이 방식은 기존 5성 히스토리 계산과 같은 관점이다. 저장된 기록 이전에 이미 pity가 쌓여 있던 경우 첫 획득 비용은 저장된 히스토리 기준으로만 계산된다.

## Pickup Pull Efficiency (변경 예정)

캐릭터 픽업 배너의 `avgPulls`와 `luckScore`는 전체 5성 획득이 아니라 한정 픽업 캐릭터 획득을 기준으로 계산한다. `expectedPulls = 80`은 픽뚫 비용까지 고려한 고정 기대값이며, 이번 변경에서 조정하지 않는다. 이 계산은 캐릭터별 재화 집계와 별도로 기존 배너 통계에 적용한다.

- 기록을 오래된 순서로 처리하고 각 5성의 `pity`를 주기 내에 누적한다. 픽뚫이면 누적을 유지하고, 픽업을 획득하면 주기를 완료한 뒤 누적을 초기화한다.
- `avgPulls` = 완료된 픽업 획득 주기의 총 뽑기 횟수 / 픽업 획득 개수.
- `luckScore` = (`expectedPulls` × 픽업 획득 개수) / 완료된 픽업 획득 주기의 총 뽑기 횟수 × 100. 두 지표는 같은 완료 주기를 사용하며 100이 기대값 기준이다.
- 마지막 픽업 이후의 픽뚫과 현재 pity는 미완료 주기이므로 두 지표에서 제외한다. 픽업 획득이 없으면 평균은 계산 불가로 표시하고 운 점수도 평가하지 않는다.
- 예: 60회에 픽뚫, 추가 70회에 픽업이면 `avgPulls = 130`, `luckScore = 80 / 130 × 100 ≈ 61.54`이다.
- 픽뚫이 없는 배너는 모든 5성 획득을 완료 주기로 취급한다. `fiveStars`, `actualRate`, 현재 pity는 기존 전체 5성 기준을 유지한다. 평균과 운 점수의 표시 문구는 배너의 획득 대상에 맞춘다.
- 저장된 기록 이전의 pity나 픽뚫은 복원하지 않는다. 첫 완료 주기도 저장된 히스토리 안에서 확인되는 비용만 사용한다.

### Current Implementation Gap

- `crates/wuwa-tracker-core/src/stats.rs`: 현재 `avgPulls`는 전체 5성의 `pity` 합을 전체 5성 개수로 나눈다. `luckScore`는 픽뚫 비용을 누적하지만 마지막 픽뚫로 끝난 미완료 주기도 기대값 1회분과 함께 포함한다. 두 계산 모두 완료된 픽업 획득 주기를 기준으로 변경해야 한다.
- `crates/wuwa-tracker-core/src/config.rs`: 캐릭터 픽업 배너의 고정 기준값 80을 유지한다.
- `crates/wuwa-tracker-types/src/lib.rs`: 픽업 획득이 없을 때 평균과 운 점수를 평가할 수 없다는 상태를 표현해야 한다.
- `crates/wuwa-tracker-webui/src/app.rs`: 현재 전체 5성 평균 문구와 `hasFiveStar` 기반 표시를 픽업 획득 기준으로 변경해야 한다. 관련 문구는 `locales/ui/ko.json`, `locales/ui/en.json`에 반영한다.

### Verification Criteria

- 픽뚫 60회 후 픽업 70회 획득 시 평균 130회와 운 점수 약 61.54를 검증한다.
- 완료된 픽업 이후 미완료 픽뚫 또는 현재 pity를 추가해도 두 지표가 바뀌지 않아야 한다.
- 픽뚫만 있고 픽업이 없는 경우 평균 및 운 점수를 평가하지 않아야 한다.
- 픽뚫이 없는 배너의 5성 평균 및 운 점수 계산은 유지해야 한다.

## UX Flow

새 페이지는 현재 대시보드와 같은 player 데이터를 공유한다.

```mermaid
flowchart LR
    Dashboard["Dashboard"] --> Characters["Characters"]
    Characters --> Detail["Character Detail"]
    Detail --> Characters
```

### Navigation

- Header 또는 control panel 아래에 간단한 탭을 둔다.
- 탭은 `Dashboard`, `Characters` 두 개만 둔다.
- router dependency는 추가하지 않고 `RwSignal<Page>` 같은 작은 상태로 전환한다.

### Characters Page

- 뽑기 히스토리 기준 5성 캐릭터 목록을 보여준다.
- 기본 정렬은 5성 캐릭터, 획득 횟수 많은 순, 최근 획득 순으로 한다.
- 각 행 또는 카드에는 이름, rarity, 획득 횟수, 총 소모 별의소리를 표시한다.
- 검색/필터는 1차 구현에서 제외한다.

### Character Detail

- 상단에 캐릭터 이름과 rarity를 표시한다.
- 핵심 지표는 `총 소모 별의소리`, `획득 횟수`, `등장 배너 수`로 제한한다.
- 상세 획득 기록은 기존 대시보드가 이미 제공하므로 1차 구현에서는 생략한다.
- 목록으로 돌아가는 버튼을 제공한다.

## Implementation Steps

1. `docs/features/character-stats.md`로 계획을 확정한다.
2. WebUI에 `Page`와 `selected_character_id` 상태를 추가한다.
3. `Stats` 배열을 입력받아 `Vec<CharacterSummary>`를 반환하는 순수 함수를 추가한다.
4. 해당 순수 함수에 최소 단위 테스트를 추가한다.
5. `CharactersPage`와 `CharacterDetail` 컴포넌트를 추가한다.
6. 기존 config 응답에 `resourceTypes.character`를 추가한다.
7. `locales/ui/ko.json`, `locales/ui/en.json`에 필요한 UI 문구만 추가한다.
8. `cargo fmt`, `cargo test -p wuwa-tracker-webui`, `cargo check --target wasm32-unknown-unknown -p wuwa-tracker-webui`로 확인한다.

## Decisions

- 캐릭터/무기 구분은 `Record.resource_type`과 game locale의 `character` 값을 비교한다. 한국어는 `공명자`, 영어는 `Resonator`를 사용한다.
- 돌파 표기는 게임 내 관례처럼 duplicate 수를 기준으로 한다. 구현은 `copies.saturating_sub(1)`이다.
- 캐릭터 풀과 상세 통계는 5성 캐릭터만 대상으로 한다.
- CLI는 `db characters <player-id>`로 요약 테이블만 제공하고, 배너별 기록 수는 `db banners <player-id>`로 제공한다.

## Acceptance Criteria

- 기존 대시보드 화면은 현재 기능을 유지한다.
- 캐릭터 페이지에서 저장된 히스토리 기반 5성 캐릭터 목록을 볼 수 있다.
- 캐릭터 상세에서 획득 횟수, duplicate 기준 돌파 표기, 총 소모 별의소리를 볼 수 있다.
- 새 기능은 기존 `StatsResponse`와 config 응답의 resource type 값만 사용하며 store 포맷을 변경하지 않는다.
- WebUI 단위 테스트가 캐릭터별 획득 횟수와 소모 별의소리 계산을 검증한다.
