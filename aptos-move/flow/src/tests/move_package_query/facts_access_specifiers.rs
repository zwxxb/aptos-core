// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Move 2 `reads`/`writes` access specifiers must surface as declared
/// capabilities instead of silently vanishing (which previously made a
/// `writes Config` function look fully pure with `effectsComplete: true`).
#[tokio::test]
async fn move_package_query_facts_access_specifiers() {
    let pkg = common::make_package("facts_access_specifiers", &[(
        "acc",
        "module 0xCAFE::acc {
    struct Config has key { v: u64 }
    struct Store<phantom T> has key { v: u64 }

    fun write_only(): u64 writes Config {
        42
    }

    fun read_generic(): u64 reads Store<u64> {
        7
    }

    fun wild(): u64 reads *(*) {
        1
    }

    fun legacy_generic(a: address): u64 acquires Store {
        borrow_global<Store<u64>>(a).v
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
