// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// `isLambdaLifted` must tag only compiler-generated `__lambda__<n>__...`
/// functions, not user functions whose names merely contain the marker.
#[tokio::test]
async fn move_package_query_facts_lambda_tag() {
    let pkg = common::make_package("facts_lambda_tag", &[(
        "tag",
        "module 0xCAFE::tag {
    fun apply(f: |u64|u64, x: u64): u64 { f(x) }

    public fun run__lambda__step(x: u64): u64 { x }

    public fun real_closure(c: u64): u64 {
        apply(|y| y + c, 10)
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
