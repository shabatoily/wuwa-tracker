# Wuwa Tracker

## Architecture

- crates/wuwa-tracker-core: Rust domain logic building blocks, scanner, stats, merge, local store
- crates/wuwa-tracker-types: Shared domain models and serialized API contracts
- crates/wuwa-tracker-app: Rust desktop app, application service layer, and optional `serve` command
- crates/wuwa-tracker-webui: Leptos CSR frontend compiled to WebAssembly
- locales: embedded JSON locale resources

## Global Rules

1. 코드 내, 주석 제외 한글 사용 금지
2. Rust 코드는 `cargo fmt` 표준 포맷을 유지
3. 외부 통신은 Rust HTTP client 계층으로만 격리

## Build Guide

- Rust workspace는 `cargo`를 사용
- frontend webui는 `cargo`, `trunk`를 사용
- 빌드 관련 커맨드는 [Makefile](./Makefile)을 참조

## Codebase Rules

Rust 언어에서 권장하는 표준을 준수

- Formatter: rustfmt
- Build/Test: cargo check, cargo test
- Frontend Check: cargo check --target wasm32-unknown-unknown

### Comments

- 코드 주석은 한글로 작성합니다. 에러 메시지, 로그/콘솔 출력은 제외합니다.
- 고유명사 혹은 한글로 번역 시 오히려 의미 해석이 어색해지는 경우는 판단하에 영어로 작성합니다.
- 주석은 코드만으로 드러나지 않는 이유, 비즈니스 규칙, 외부 제약, 불변 조건을 설명할 때만 작성합니다.
- 코드의 동작이나 제어 흐름을 말로 반복하는 주석과 함수명, 변수명을 다시 설명하는 Rustdoc은 작성하지 않습니다.
- 직관적이지 않은 계산, 정렬 우선순위, fallback 순서, 호환성 처리, 성능상 선택은 변경 이유와 유지해야 할 조건을 간결하게 기록합니다.
- 공개 API의 계약이 이름과 타입만으로 충분하지 않을 때 `///` Rustdoc을 작성합니다. 필요할 때만 `# Errors`, `# Panics`, `# Safety`, `# Examples` 섹션을 추가합니다.
- 타입, 함수, 프로토콜 등의 식별자는 백틱 또는 Rustdoc 링크로 표기합니다.
- 코드 변경으로 주석이 부정확해지면 같은 변경에서 수정하거나 삭제합니다.
- 주석 처리된 코드, 변경 이력, 근거 없는 TODO는 남기지 않습니다.

### Documentation Style

- 문서와 PR 본문은 간결한 개조식으로 작성.
- `~합니다`, `~해야 합니다`, `~하다`보다 `~함`, `~했음` 또는 의미가 완결되는 짧은 표현 사용.
- `추가`, `수정`, `정의`, `확인`만으로 의미가 충분하면 불필요한 서술어 생략.
- 예시: `~으로 정의합니다` → `~으로 정의`, `테스트를 통과했습니다` → `테스트 통과`.
- 조건, 제약, 실패 사유 등 판단에 필요한 정보는 축약 시에도 유지.

### Branch Strategy

- 기본 브랜치는 `main`으로 유지합니다.
- `main`은 릴리즈 가능한 안정 브랜치로 관리합니다.
- `develop`은 다음 릴리즈 후보를 통합하는 상시 개발 브랜치로 유지합니다.
- 일반 기능 작업은 `feature/<description>` 브랜치에서 진행하고 `develop`으로 PR을 생성합니다.
- 버그 수정은 `fix/<description>` 브랜치에서 진행합니다.
- 긴급 수정이 필요한 경우 `main`에서 `hotfix/<description>` 브랜치를 만들고, 머지 후 `develop`에도 반영합니다.
- `main` 머지는 릴리즈 생성 기준이므로 직접 push 대신 PR을 사용합니다.

### Commit Message

- 커밋 메시지는 Conventional Commits 형식을 사용합니다.
- 메시지 형식은 `<type>: <summary>`를 기본으로 합니다.
- 기능 추가는 `feat`, 버그 수정은 `fix`, CI/워크플로우 변경은 `ci`, 문서 변경은 `docs`, 테스트 변경은 `test`, 리팩터링은 `refactor`, 기타 관리 작업은 `chore`를 사용합니다.
- 브랜치 성격과 변경 내용을 기준으로 적절한 prefix를 선택합니다.
- 소스 코드와 실제 빌드 산출물에 전혀 영향이 없는 변경은 CI/CD 파이프라인을 건너뛸 수 있도록 커밋 메시지 앞에 `[skip ci]`를 붙입니다.
- 예시:
  - `feat: centralize runtime config defaults`
  - `fix: prepare embedded webui before CI tests`
  - `ci: add release workflow`
  - `[skip ci] docs: update agent workflow guidance`

## Workflows

### Documentation Analysis and DESIGN.md Synthesis

이 워크플로우의 목적은 프로젝트의 기존 문서, 주석, 그리고 실제 코드를 단계적으로 분석하여 실제 구현과 일치하는 최신화된 `DESIGN.md` 파일을 생성하거나 수정하는 것입니다.

#### Analysis Principles

- Source of Truth: 문서와 코드가 상충할 경우, 실제 동작하는 코드를 최우선 순위로 둡니다.
- Incremental Update: 기존 `DESIGN.md`가 존재할 경우, 무조건적인 덮어쓰기보다 변경 사항을 추적하여 논리적으로 수정합니다.
- Evidence-based: 모든 설명은 프로젝트 내의 물리적 파일 경로와 연결되어야 합니다.

#### Execution Steps

1. Document Analysis
   - 대상: 프로젝트 루트 및 `docs/` 디렉토리 내의 모든 `.md` 파일.
   - 목표: 프로젝트의 설계 의도, 비즈니스 로직의 배경, 아키텍처 가이드를 파악합니다.
   - 결과: 가상의 설계 지도를 생성합니다.

2. Spec Extraction
   - 대상: 소스 코드 전체 (`src/` 등).
   - 목표: 클래스, 메서드 상단에 기술된 Javadoc, Docstring 등을 추출합니다.
   - 비교: Step 1에서 파악한 설계 의도가 실제 인터페이스 명세(주석)와 일치하는지 대조합니다.

3. Code Deep-Dive
   - 대상: 실제 비즈니스 로직 구현체 (`.java`, `.py`, `.ts`, `.go` 등).
   - 목표: 실제 호출 흐름, 데이터 저장 방식, 예외 처리 로직을 분석합니다.
   - 대조 검증:
     - 문서에는 존재하지만 코드에는 없는 기능 식별.
     - 코드에는 구현되었으나 문서/주석에 누락된 로직 식별.
     - 기존 분석 결과와 실제 코드 간의 논리적 모순 기록.

4. DESIGN.md Synthesis
   - 대상: 프로젝트 루트의 `DESIGN.md`.
   - 구성:
     - Architecture Overview: 프로젝트의 전체적인 구조.
     - Component/Module Details: 분석된 실제 모듈별 역할 정의.
     - Implementation vs Design: 문서와 실제 코드 간의 주요 차이점이 있는 경우 기록.
     - Updated Date: 분석 완료 시간 및 기준 커밋 정보가 가능할 경우 기록.

#### Documentation Agent Requirements

- 분석 중 발견된 모든 불일치(Discrepancy)는 무시하지 말고 `DESIGN.md`에 Notes 또는 Known Limitations 섹션으로 기록할 것.
- 가급적 표준 기술 용어(Design Patterns, Architecture Styles)를 사용하여 전문성을 유지할 것.
- 출력물(`DESIGN.md`)의 언어 설정은 사용자의 별도 지시가 없다면 프로젝트의 주 사용 언어를 따를 것.

### Commit Workflow

이 워크플로우는 현재 수정 내용에 맞는 브랜치를 생성하고, 모든 현재 변경 사항을 스테이징한 뒤 설명적인 커밋 메시지로 커밋하는 절차입니다.

1. Prepare Branch
   - 현재 브랜치와 변경 내용을 확인합니다.
   ```bash
   git status --short
   git branch --show-current
   ```
   - 변경 성격에 맞는 브랜치 prefix를 선택합니다.
     - 기능 추가: `feature/<description>`
     - 버그 수정: `fix/<description>`
     - 긴급 수정: `hotfix/<description>`
     - 문서 변경: `docs/<description>`
     - 테스트 변경: `test/<description>`
     - 리팩터링: `refactor/<description>`
     - 기타 관리 작업: `chore/<description>`
   - 이미 적절한 작업 브랜치에 있다면 새 브랜치를 만들지 않습니다.
   - `main` 또는 `develop`에 있다면 수정 내용에 맞는 브랜치를 생성한 뒤 작업합니다.
   ```bash
   git switch -c "<type>/<description>"
   ```

2. Audit Packages
   ```bash
   make ci
   ```

3. Build Check
   ```bash
   make build
   ```

4. Stage All Changes
   ```bash
   git add .
   ```

5. Analyze Changes
   ```bash
   git diff --cached
   ```

6. Generate and Commit
   - Conventional Commits 형식의 전문적인 메시지를 생성하여 커밋합니다.
   - 소스 코드와 실제 빌드 산출물에 전혀 영향이 없는 문서, 주석, 메타데이터성 변경은 `[skip ci] <commit_message>` 형식을 사용합니다.
   ```bash
   git commit -m "<ai_generated_message>"
   ```
   ```bash
   git commit -m "[skip ci] docs: update agent workflow guidance"
   ```

7. Push
   - 필요할 경우 변경 사항을 push합니다.
   ```bash
   git push
   ```

8. Prepare PR Description
   - 커밋·push 완료 후 PR 생성 여부와 무관하게 최종 응답에 PR 본문 초안 포함.
   - PR 본문만 요청받은 경우에도 동일한 절차 적용.
   - 바로 복사할 수 있도록 본문 전체를 `markdown` 코드 블록으로 출력.
   - 출력 전 아래 섹션 구성과 `Documentation Style` 준수 여부 확인.
   - `Summary` 섹션은 PR의 전체 의도를 간결하게 설명합니다.
   - `Changes` 섹션은 구체적인 구현 변경 사항을 작성합니다.
   - `Verification` 섹션은 확인한 명령을 포함합니다.
   - 의미 있는 커밋 단위로 나눈 경우 `Commits`를 포함합니다.
   - 호환성, 리뷰 집중 영역, 마이그레이션, 릴리즈 고려 사항이 있을 때만 `Review Notes`를 포함합니다.
   - 각 bullet은 집중된 내용으로 작성하고 긴 서술형 문단은 피합니다.

   ```markdown
   ## Summary

   <One short paragraph explaining the purpose of the PR.>

   ## Changes

   - ...

   ## Commits

   - `<commit message>`

   ## Verification

   - [x] `...`

   ## Review Notes

   - ...
   ```
