# 의존성 경고 점검

- 점검일: 2026-10-28
- RustSec 감사 경고: 기존 6건에서 3건으로 감소
- 경고 무시 설정·컴파일 경고 억제 추가 없음

## 해결

| 경고 | 조치 |
| --- | --- |
| `tokio` / RUSTSEC-2026-0386 | `1.54.1`로 업데이트. 수정 버전의 I/O 안전성 처리 적용 |
| `basic-toml` / RUSTSEC-2026-0374 | Askama에서 사용하지 않는 `config` 기능 비활성화, 의존성 제거 |
| `proc-macro-error2` / RUSTSEC-2026-0173 | Leptos·RSTML 계열 업데이트로 의존성 제거 |
| `proc-macro-error2` Rust 향후 호환성 경고 | 해당 crate 제거로 원인 제거 |

- Askama는 기존 파일 템플릿에 필요한 `derive`, `std`만 활성화
- Leptos 매크로의 `syn` 버전 호환성을 위해 `server_fn_macro_default`도 함께 업데이트
- 관련 패키지와 필수 하위 의존성만 업데이트. workspace 전체 일괄 업데이트 제외

## 남은 상위 의존성 제약

| 경고 | 유입 경로 | 남은 제약 |
| --- | --- | --- |
| `paste` / RUSTSEC-2024-0436 | Leptos·Tachys·Reactive Graph | 현재 호환 릴리스에서 유지보수 중단 crate 사용. 상위 라이브러리의 대체 필요 |
| `proc-macro-error` / RUSTSEC-2024-0370 | Linux Tauri → GTK3 → `glib-macros`, `gtk3-macros` | 현재 GTK3 계열의 전이 의존성. 해당 crate의 수정 릴리스 없음 |
| `glib` / RUSTSEC-2024-0429 | Linux Tauri → GTK3/WebKitGTK → `glib 0.18.5` | 수정된 `glib >=0.20`은 현재 GTK3 의존 범위 `0.18`과 비호환 |

- Linux GUI 의존성도 `Cargo.lock` 감사 대상. macOS 빌드에서 사용하지 않는다는 이유로 경고 제외하지 않음
- 세 경고는 미해결 상태로 유지. 플랫폼 지원 삭제·전이 의존성의 대규모 vendoring·무시 목록 추가는 시행하지 않음
- GTK3·Leptos 상위 의존성 변경 시 재점검 필요
- Linux GUI 실행·교차 빌드는 이번 macOS 검증에 미포함

## 확인 명령

- `make ci`, `make build` 통과. 단위 테스트 22개 통과
- CLI 파일 입력으로 HTML 리포트 생성 및 템플릿·원본 이름 출력 확인
- 빌드 출력에서 `proc-macro-error2` 향후 호환성 경고 미발생 확인

```bash
cargo tree -i paste
cargo tree --target all -i proc-macro-error
cargo tree --target all -i glib
make ci
make build
```

## 근거

- [Tokio I/O 안전성](https://rustsec.org/advisories/RUSTSEC-2026-0386.html)
- [basic-toml 유지보수 중단](https://rustsec.org/advisories/RUSTSEC-2026-0374.html)
- [proc-macro-error2 유지보수 중단](https://rustsec.org/advisories/RUSTSEC-2026-0173.html)
- [paste 유지보수 중단](https://rustsec.org/advisories/RUSTSEC-2024-0436.html)
- [proc-macro-error 유지보수 중단](https://rustsec.org/advisories/RUSTSEC-2024-0370.html)
- [glib 수정 버전](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)
