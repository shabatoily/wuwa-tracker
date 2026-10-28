# 에코 세트 목록 API

- 구현일: 2026-10-28
- HTTP: `GET /api/echo-sets?lang={lang}`
- Tauri: `list_echo_sets` 명령
- 언어: `ko`, `en` 지원. 요청별 `lang` 사용, 생략 시 `Config.language` 적용. 미지원 값은 오류 반환

## 수집·캐시

- GUI 시작 및 `serve` 시작 시 `GET https://api-v2.encore.moe/api/{language}/echo`를 `ko`, `en` 각각 한 번 병렬 호출
- 일반 CLI 명령 실행 시 수집 제외
- 에코별 `FetterGroups`를 세트 ID로 중복 제거하고 ID 오름차순 정렬
- 세트 효과는 필요 개수 오름차순 정렬. 동일 ID의 세트 내용 불일치·빈 결과·잘못된 필수 값은 수집 실패 처리
- 원본 에코 목록은 수집 후 폐기. 정규화한 세트 목록만 메모리에 보관
- 서비스 복제본끼리 캐시 공유. 초기화 중복 호출도 언어별 한 번만 수집
- 시작 시 언어별 수집 실패 기록 후 기존 앱·서버 기능은 계속 사용 가능. 한 언어 실패가 다른 언어 캐시를 무효화하지 않음
- 수집 요청 제한 시간 10초. 서버 수신·GUI 표시 전에 초기화 완료 대기
- 목록 조회·표시 언어 전환에서는 성공·실패 여부와 관계없이 외부 API 호출 없음
- 재시도·주기 갱신·redb 영속 저장 제외. 실패 후 재수집은 앱·서버 재시작 필요
- 조사 시점 원본 약 584KB, 중복 제거 세트 37개·원본 세트 JSON 약 23KB로 메모리 저장 선택

## 요청별 언어

- HTTP query `lang`, Tauri 명령 인자 `lang`으로 전달
- 적용: 에코 세트, 통계 조회, track/upload 통계 응답, config 자원 종류, UI 번역 API, 리포트
- 미지원 언어: HTTP `400`, `errorKey: "err.unsupported_language"`
- 전역 표시 언어 상태·`/api/language`·`get_language`·`set_language` 제거
- 다른 클라이언트의 언어 선택과 독립적으로 캐시 조회
- WebUI는 내장 UI 번역·localStorage 사용. 언어 전환 시 활성 플레이어 통계를 새 `lang`으로 재조회
- `get_i18n` API는 유지하되 현재 WebUI에서는 직접 호출하지 않음
- 통계의 `gachaName` 및 캐릭터 요약 배너 제목은 내장 게임 locale로 현지화
- 저장 기록의 캐릭터·무기 이름과 종류는 원본 유지. ID 기반 이름 번역 제외
- 캐릭터 집계는 한국어·영어 원본 종류를 모두 인식. 표시 언어 또는 마지막 수집 locale 변경으로 수치가 달라지지 않도록 처리
- 다른 원본 언어는 기존 수집 locale 판별에 의존. 전체 언어의 종류 정규화·저장 스키마 변경은 제외
- `Accept-Language` 해석·언어 협상 제외

```bash
curl 'http://127.0.0.1:3000/api/echo-sets?lang=en'
curl 'http://127.0.0.1:3000/api/stats/player-id?lang=en'
curl 'http://127.0.0.1:3000/api/i18n?lang=en'
```

## 응답

```json
{
  "success": true,
  "language": "ko",
  "sets": [
    {
      "id": 9,
      "name": "끊임없는 잔향",
      "icon": "https://api.encore.moe/resource/Data/Game/Aki/UI/UIResources/Common/Image/IconElementAttri/T_IconElementAttriAttack.webp",
      "effects": [
        {
          "id": 17,
          "requiredCount": 2,
          "description": "공격력이 10% 증가된다"
        }
      ]
    }
  ]
}
```

- 예시는 응답 일부만 발췌
- `requiredCount`: 원본 `Fetters[].Key`의 세트 활성화 필요 개수
- `description`: 원본 `EffectDescription` 보존. 태그 및 `{0}` 같은 미치환 placeholder가 포함될 수 있음
- 세트 목록은 원본 에코 목록에 포함된 세트 기준. 원본에 누락된 세트 복원 기능 없음
- 초기화 전 또는 수집 실패 시 HTTP `503`, `success: false`, `error: "echo set data is unavailable"`, `errorKey: "app.network_error"`
- Tauri 수집 실패 응답은 기존 `AppError` 직렬화 규칙에 따라 오류 문자열 반환

## 구현 위치

- `crates/wuwa-tracker-core/src/encore.rs`: 외부 HTTP 호출, 응답 검증·세트 추출
- `crates/wuwa-tracker-types/src/lib.rs`: 공용 응답 모델
- `crates/wuwa-tracker-app/src/service.rs`: 시작 시 초기화, 공유 메모리 캐시, 목록 조회
- `crates/wuwa-tracker-app/src/http.rs`, `api.rs`: HTTP·Tauri 진입점
- `crates/wuwa-tracker-app/src/lib.rs`, `main.rs`: GUI 초기화 및 명령 등록

## 검증 기준

- 세트 중복 제거·정렬 및 효과 설명 보존
- 빈 결과·잘못된 효과·세트 내용 충돌 거부
- 초기화 동시 호출과 실패 후 중복 호출에도 언어별 수집 한 번
- 요청 언어에 해당하는 캐시 반환, 미지원 언어 거부
- 한 언어 캐시 수집 실패 시 다른 언어로 전환해 정상 조회
- 목록 반복 조회 시 캐시 공유 및 외부 호출 없음
- 캐시 미준비 시 HTTP `503`

## 검증 결과

- `make ci` 통과: 단위 테스트 22개, CLI·WASM·workspace 검사, Clippy 포함
- `make build` 통과: GUI·workspace 및 CLI 전용 빌드
- 실제 원격 데이터로 임시 HTTP 서버 실행 후 목록 2회 조회: 세트 37개, 동일 응답 확인
- 정규화된 실제 HTTP 응답 약 20KB
- 실제 HTTP 한국어·영어 동시 조회: 배너 제목·config 종류·번역·에코 언어 분리 확인
- 한국어 원본 업로드 후 영어 통계·JSON 리포트 조회: 원본 이름·획득 수·소모 재화 유지 확인
- 언어별 에코 37개 반환, 조회 후에도 시작 시 수집 로그 총 2건 유지
- 미지원 `lang`은 `400`, 제거된 `/api/language`는 `404` 확인
- 의존성 감사의 허용된 경고 6건 및 `proc-macro-error2` 향후 호환성 경고 유지
