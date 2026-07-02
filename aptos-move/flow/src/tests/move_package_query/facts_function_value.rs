// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// A function invoking a function-value parameter must not claim purity:
/// `invokesFunctionValues: true`, `effectsComplete: false`.
#[tokio::test]
async fn move_package_query_facts_function_value() {
    let pkg = common::make_package("facts_function_value", &[(
        "fv",
        "module 0xCAFE::fv {
    public fun foo(bar: |u64|u64): u64 {
        bar(1)
    }

    public fun pure_leaf(x: u64): u64 {
        x + 1
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
