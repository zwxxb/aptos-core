// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_facts_module() {
    let pkg = common::make_package("facts_module", &[
        (
            "core",
            "module 0xCAFE::core {
    friend 0xCAFE::helper;

    const MAGIC: u64 = 42;
    const NAME: vector<u8> = b\"core\";
}",
        ),
        ("helper", "module 0xCAFE::helper { }"),
    ]);
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
