// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// The call_graph query must fail on a package with compilation errors instead
/// of silently emitting a shrunken graph (get_called_functions() returns empty
/// for error-failed functions, hiding edges without any signal).
#[tokio::test]
async fn move_package_query_call_graph_compile_error() {
    let pkg = common::make_package("call_graph_compile_error", &[(
        "broken",
        "module 0xCAFE::broken {
    struct R has key { v: u64 }
    struct SomethingElse has key { v: u64 }

    // Error: declared acquires is non-empty but misses `R` (read below).
    public fun bad(): u64 acquires SomethingElse {
        borrow_global<R>(@0xCAFE).v
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "call_graph" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
