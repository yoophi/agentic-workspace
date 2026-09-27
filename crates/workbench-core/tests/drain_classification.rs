//! 044 T008: 비우기 입구 분류표(`specs/044-standalone-server/contracts/drain-classification.md`)와 코드의 대조. 문서 표를
//! 파싱해 모든 operation이 정확히 한 행이고, 분류가 core `drain_class`와 같으며, query는 Q임을 단정한다(research R7).

use std::collections::HashMap;

use workbench_core::application::drain::{drain_class, DrainClass};
use workbench_protocol::{operations::spec_for, OperationId, OperationKind};

fn table() -> HashMap<String, (String, String)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/044-standalone-server/contracts/drain-classification.md");
    let doc = std::fs::read_to_string(&path).expect("drain classification contract");
    let mut rows = HashMap::new();
    for line in doc.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // | `op` | kind | class | 까닭 |
        if cells.len() < 5 || !cells[1].starts_with('`') {
            continue;
        }
        let op = cells[1].trim_matches('`').to_owned();
        let previous = rows.insert(op.clone(), (cells[2].to_owned(), cells[3].to_owned()));
        assert!(previous.is_none(), "{op}: appears twice in the contract table");
    }
    rows
}

fn letter(class: DrainClass) -> &'static str {
    match class {
        DrainClass::Query => "Q",
        DrainClass::Control => "C",
        DrainClass::Continuation => "K",
        DrainClass::NewWork => "N",
    }
}

#[test]
fn every_operation_has_exactly_one_row_matching_the_code() {
    let rows = table();
    assert_eq!(rows.len(), OperationId::ALL.len(), "table rows vs operations");
    for id in OperationId::ALL {
        let (kind, class) = rows
            .get(id.as_str())
            .unwrap_or_else(|| panic!("{id}: missing from the contract table"));
        let spec_kind = match spec_for(id).kind {
            OperationKind::Query => "query",
            OperationKind::Command => "command",
        };
        assert_eq!(kind, spec_kind, "{id}: kind column");
        assert_eq!(class, letter(drain_class(id)), "{id}: class column vs drain_class");
        if spec_for(id).kind == OperationKind::Query {
            assert_eq!(drain_class(id), DrainClass::Query, "{id}: queries are Q");
        }
    }
}
