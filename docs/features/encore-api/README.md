# Encore API 조사

- 조사일·명세 다운로드일: 2026-10-27 (Asia/Seoul)
- 상태: 조사 완료, [에코 세트 목록 API](./echo-sets.md) 구현
- 목적: 공명자·에코 메타데이터 및 에코 강화 옵션 수치의 공개 API 제공 여부 확인
- 범위: encore.moe 공개 API. 부옵션 출현 확률은 요구 범위에서 제외

## 자료

- [OpenAPI 스냅샷](./openapi.json): 공개 원본을 수정 없이 저장
- 원본: <https://api-v2.encore.moe/openapi.json>
- API 문서: <https://api-v2.encore.moe/_docs/scalar>
- 사이트의 API 안내: <https://encore.moe/about>
- 명세: OpenAPI `3.1.0`, API 버전 `2.0.0`, 경로 38개
- 스냅샷 SHA-256: `a64ab4cecffddf7624775e178c077fe6322c161eb85f98d59e1bf1a6dca6498d`

## 결론

| 대상 | 확인 결과 | 근거 |
| --- | --- | --- |
| 공명자 목록·상세 | 공개 API로 조회 가능 | 목록·앙코 상세의 한국어 JSON 응답 확인 |
| 에코 목록·상세 | 공개 API로 조회 가능 | 에코 목록·상세의 한국어 JSON 응답 확인 |
| 에코 스킬·세트 효과 | 상세 응답에서 조회 가능 | `Skill`, `FetterGroupDetails`, `FetterDetails` 확인 |
| 레벨별 주옵션 수치표 | 확인한 응답에서 미제공 | `MainProp.RandGroupId`, `LevelUpGroupId` 참조 값만 확인 |
| 부옵션별 가능한 수치 집합 | 공개 API에서 제공 경로 미확인 | 명세·경로 목록에 전용 경로 없음. 확인한 에코 응답에도 수치 배열 없음 |

- 현재 확인한 API만으로 부옵션 수치 집합 확보 가능 여부 확정 불가
- 미문서화 경로·원본 데이터 파일까지 조사한 결과는 아님. 서버 내부에 데이터 자체가 없다는 의미로 해석 금지
- wuthering.gg 배포 데이터 추출은 이번 문서의 연동 방안에서 제외

## 접속 정보

- 기본 서버: `https://api-v2.encore.moe/api`
- 명세상 보조 서버: `https://api.encore.moe` — 이번 조사에서 보조 서버 동작 미검증
- 언어: 경로의 `{lang}` 사용. 한국어는 `ko`
- 명세 지원 언어: `en`, `zh-Hans`, `zh-Hant`, `ja`, `ko`, `de`, `es`, `fr`, `id`, `pt`, `ru`, `th`, `vi`
- 확인한 GET 요청은 API 키·로그인 없이 성공. 명세에 전역 `security` 요구 없음
- 실제 게임 계정의 보유 공명자·장착 에코 조회 기능이 아닌 게임 데이터베이스 조회 API

## 주요 경로

| 메서드 | 경로 | 용도 |
| --- | --- | --- |
| GET | `/{lang}` | 해당 언어의 경로 목록 |
| GET | `/{lang}/character` | 공명자 목록 |
| GET | `/{lang}/character/{id}` | 공명자 상세 |
| GET | `/{lang}/echo` | 에코 목록 |
| GET | `/{lang}/echo/{id}` | 에코 상세 |

```bash
curl --fail --location 'https://api-v2.encore.moe/api/ko/character'
curl --fail --location 'https://api-v2.encore.moe/api/ko/character/1203'
curl --fail --location 'https://api-v2.encore.moe/api/ko/echo'
curl --fail --location 'https://api-v2.encore.moe/api/ko/echo/6000038'
```

## 실제 응답과 명세의 차이

- 공명자 목록: 실제 응답의 `roleList` 배열 사용
- 공명자 상세: 앙코 `1203`에서 `Name.Content`, `Properties`, `Skills`, `SkillTree`, `ResonantChain`, `Breaches` 확인
- 에코 목록: 명세의 `EchoList`는 `phantomsList`로 정의되어 있으나, 실제 확인 응답은 `Echo` 배열
- 에코 상세: 명세는 `Id`, `Name` 등을 설명하지만, `6000038` 실제 응답은 `ItemId`, `MonsterId`, `MonsterName` 등을 포함
- 에코 상세 실제 응답에 `MainProp`, `LevelUpGroupId`, `FetterGroupDetails`, `FetterDetails` 등 명세에 열거되지 않은 필드 포함
- 명세만으로 클라이언트 모델을 확정하지 않고 실제 응답과 대조 필요
- 원본 응답 파일은 저장하지 않음. 위 필드 정보는 조사 시점의 샘플 기준

## 연동 시 고려 사항

- 외부 통신은 프로젝트 규칙에 따라 Rust HTTP client 계층으로 격리
- 공명자·에코 메타데이터는 갱신 시 수집 후 캐시하는 방식 검토
- 부옵션 수치 집합은 별도 근거 확보 전까지 이 API의 제공 기능으로 취급하지 않음
- 참조 그룹 ID만으로 강화 수치 복원 불가. 대응 테이블 확보 필요
- 대량 호출 제한, 캐시 정책, 재배포 허용 범위, Beta 데이터 선택 방식 미검증
- 서비스·데이터 변경 가능성: [이용약관](https://encore.moe/terms) 참고

## 스냅샷 갱신·검증

프로젝트 루트에서 실행:

```bash
curl --fail --location --silent --show-error \
  'https://api-v2.encore.moe/openapi.json' \
  -o docs/features/encore-api/openapi.json
python3 -m json.tool docs/features/encore-api/openapi.json > /dev/null
shasum -a 256 docs/features/encore-api/openapi.json
```

- 갱신 시 다운로드일·버전·경로 수·해시 및 실제 응답과의 차이 재확인
- 이번 변경은 문서·명세 보관만 포함
- 검증: JSON 파싱, `git diff --check`, `make ci`, `make build` 통과
- `make ci`: 단위 테스트 18개 통과. 의존성 감사의 허용된 경고 6건 출력
- `proc-macro-error2`의 Rust 향후 호환성 경고 출력. 의존성 변경은 이번 범위에서 제외
