// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Native functions have no analyzable body: `effectsComplete: false`.
#[tokio::test]
async fn move_package_query_facts_native() {
    let pkg = common::make_package("facts_native", &[(
        "nat",
        "module 0xCAFE::nat {
    native fun ffi(x: u64): u64;

    public fun wrapper(x: u64): u64 {
        ffi(x)
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
