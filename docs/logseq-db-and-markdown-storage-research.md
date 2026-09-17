# Logseq DB 전환과 Markdown 파일 그래프 유지 방법

조사일: 2026-08-11
범위: Logseq가 운영하는 공식 GitHub 저장소·문서·공지(비공식 글 제외)

## 결론

Logseq 2.x의 `Logseq`는 **DB 그래프용 제품**이고, 기존의 Markdown 파일 그래프는 별도 제품인 **Logseq OG**로 공식 분리되었다. 따라서 Markdown 파일을 원본 저장소로 계속 쓰려면 `Logseq OG`를 사용해야 한다. 기존 파일 그래프를 DB판으로 옮길 의무는 없으며, 두 앱을 나란히 설치해 각 그래프에 맞게 사용할 수 있다.

`Logseq` DB판에서 말하는 Markdown 지원(가져오기·내보내기·문법 지원)은 파일을 원본 저장소로 계속 사용하는 것과 다르다. DB 그래프의 실제 저장소는 SQLite이고, 표준 Markdown 내보내기는 속성·타임스탬프를 완전히 보존하지 못한다.

## 왜 DB 방식으로 전환했나

공식 팀의 설명은 다음 네 축으로 정리된다.

1. **협업·동기화 한계** — Markdown 파일 위에서 실시간 협업을 구현하기가 매우 어렵다. 블록 하나를 새로 만들 때 파일 전체를 다시 써야 하고, 페이지 이름을 바꾸면 그 페이지를 참조하는 모든 파일을 갱신해야 한다. 파일 시스템 복잡성은 동기화 경험과 이름 변경 성능에도 영향을 준다. [2024년 공식 DB 전환 설명](https://discuss.logseq.com/t/why-the-database-version-and-how-its-going/26744), [공식 팀의 파일 기반 한계 설명](https://discuss.logseq.com/t/database-version-too-drastic-choice/20346/5)
2. **구조화 데이터의 한계** — 파일 그래프는 DB에 비해 영속 ID, 생성·수정 시각 등 구조 데이터 표현이 제한적이다. DB판 문서는 모든 블록과 페이지에 생성·수정 타임스탬프를 둔다고 명시한다. [DB 전환 설명](https://discuss.logseq.com/t/why-the-database-version-and-how-its-going/26744), [DB 변경 문서](https://github.com/logseq/docs/blob/master/db-version-changes.md)
3. **대형 그래프의 안정성·성능** — 팀은 기존 앱의 대형 그래프 성능, 다중 클라이언트 동기화 시 데이터 손실, 신뢰하기 어려운 undo를 문제로 들며 더 안정적이고 빠른 기반을 목표로 제시했다. DB 변경 문서도 로딩·대형 그래프·대형 테이블 처리가 개선되었다고 설명한다. [DB 전환 설명](https://discuss.logseq.com/t/why-the-database-version-and-how-its-going/26744), [DB 변경 문서](https://github.com/logseq/docs/blob/master/db-version-changes.md)
4. **두 아키텍처를 한 앱에서 유지하는 비용** — 2026년 제품 분리 공지는 파일 그래프와 DB 그래프를 한 앱에 함께 유지하면 기능·버그 수정·UX 개선을 두 번 구현해야 하므로 개발이 느려지고 회귀와 사용자 혼란이 커진다고 설명한다. [공식 제품 분리 공지](https://logseq.io/page/b2ad9ce1-9cb7-4436-8083-54cb4516d324/df4dc09d-0a12-4c87-904e-22a9bf4c350a)

## 현재의 공식 제품 구분

| 용도 | 공식 제품·저장 방식 | 공식 근거 |
| --- | --- | --- |
| Markdown 파일을 원본으로 편집·Git/외부 편집기와 함께 사용 | **Logseq OG** — file-based graphs (Markdown), 저장소: [`logseq/og`](https://github.com/logseq/og) | [제품 분리 공지](https://logseq.io/page/b2ad9ce1-9cb7-4436-8083-54cb4516d324/df4dc09d-0a12-4c87-904e-22a9bf4c350a), [DB 변경 문서](https://github.com/logseq/docs/blob/master/db-version-changes.md) |
| 2.x의 DB 기능·동기화·협업 사용 | **Logseq** — database graphs, 그래프 데이터·설정은 `~/logseq/graphs/<GRAPH-NAME>/db.sqlite`에 저장 | [DB 변경 문서](https://github.com/logseq/docs/blob/master/db-version-changes.md), [DB 기능 문서](https://github.com/logseq/docs/blob/master/db-version.md) |

공식 공지는 Markdown 사용자에게 즉시 바뀌는 것은 없고 DB로의 강제 마이그레이션도 없다고 밝힌다. Logseq OG는 보안 패치와 Electron·의존성 업데이트를 계속 받지만, 신규 기능 개발의 중심은 DB판이다. 두 앱의 병행 설치도 지원 방침으로 안내한다. [공식 제품 분리 공지](https://logseq.io/page/b2ad9ce1-9cb7-4436-8083-54cb4516d324/df4dc09d-0a12-4c87-904e-22a9bf4c350a)

## Markdown 파일 방식을 계속 쓰는 방법

1. 기존 Markdown 그래프는 **Logseq OG**에서 연다. 공식 저장소의 [최신 릴리스](https://github.com/logseq/og/releases/latest)에서 설치본을 받는다.
2. DB판 `Logseq`는 DB 그래프를 시험하거나 새 DB 그래프를 만들 때만 별도로 사용한다. 같은 Mac에 두 앱을 함께 설치할 수 있다.
3. DB판의 `Import` → `File to DB graph`는 Markdown 파일 그래프를 **DB 그래프로 변환하는 선택적 가져오기**다. 이 작업을 하지 않으면 Markdown 파일 그래프를 DB 그래프로 옮기지 않는다. [DB 그래프 가져오기 문서](https://github.com/logseq/docs/blob/master/db-version.md#convert-file-graph-to-db-graph)
4. DB판에서 Markdown이 필요해도 이를 파일 그래프와 동등한 원본 저장 방식으로 간주하면 안 된다. 공식 문서는 표준 Markdown 내보내기가 블록 속성을 포함하지 않고, 타임스탬프·모든 속성을 완전히 담을 수 없다고 명시한다. [DB 내보내기 문서](https://github.com/logseq/docs/blob/master/db-version.md#graph-export)

## DB판에서의 Markdown 관련 기능: 한계

- DB판은 Markdown 파일 그래프를 가져올 수 있고, 표준 Markdown으로 내보낼 수 있다. 하지만 DB 그래프의 원본은 SQLite DB다. [DB 가져오기·내보내기 문서](https://github.com/logseq/docs/blob/master/db-version.md)
- 2026년 5월 공지의 **Markdown Mirror**는 DB 블록을 디스크의 Markdown 투영본으로 쓰는 그래프별 opt-in 기능이었다. 당시 파일에서 DB로 다시 반영하는 양방향 편집은 아직 출시되지 않았다고 명시됐다. 따라서 이 공지만으로는 파일 그래프를 대체하는 양방향 저장 방식이 출시됐다고 판단할 수 없다. [Markdown Mirror 공지](https://discuss.logseq.com/t/whats-new-with-logseq-db-may-16th-2026/35020)
- "DB판은 Markdown만 지원한다"는 문구는 DB판에서 지원하는 문서 형식(Org mode 미지원)을 뜻한다. 같은 공식 문서가 DB 그래프의 실제 데이터와 설정을 `db.sqlite`에 둔다고 설명하므로, 이를 Markdown 파일 저장 방식의 지속으로 해석해서는 안 된다. [DB 변경 문서](https://github.com/logseq/docs/blob/master/db-version-changes.md)

## 적용 판단

Markdown 파일을 직접 관리하고 Git·ripgrep·외부 편집기와 동등한 양방향 워크플로를 유지하려면 **Logseq OG를 유지**하는 것이 현재 공식 경로다. DB 기능을 시험할 경우에는 파일 그래프의 백업을 먼저 만들고, `File to DB graph` 가져오기를 별도 그래프로 수행하는 편이 안전하다. DB판 자체는 현재 beta이며, 공식 저장소도 중요한 그래프 대신 전용 테스트 그래프와 SQLite 백업을 권고한다. [Logseq DB 저장소 안내](https://github.com/logseq/logseq#-database-version)
