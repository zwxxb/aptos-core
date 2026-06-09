// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

// Exercises attribute serialization with `name = value` assignments, including
// a module-qualified name value. Each `Assign` collapses to `{ name, value }`
// on the attribute itself (no redundant `args` wrapper).
#[tokio::test]
async fn move_package_query_facts_attributes() {
    let pkg = common::make_package("facts_attributes", &[(
        "groups",
        "module 0xCAFE::groups {
    #[resource_group(scope = global)]
    struct Registry {}

    #[resource_group_member(group = 0xCAFE::groups::Registry)]
    struct Member has key { value: u64 }
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
