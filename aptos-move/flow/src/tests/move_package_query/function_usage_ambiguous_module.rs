// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use aptos_package_builder::PackageBuilder;

/// Two modules share the short name `dup` at different addresses.
/// A 2-part `dup::f` query must error with an ambiguity message instead of
/// silently picking the first match.
/// A fully-qualified `0xa::dup::f` query must succeed and disambiguate.
#[tokio::test]
async fn move_package_query_function_usage_ambiguous_module() {
    let mut builder = PackageBuilder::new("dup_test");
    builder.add_alias("a", "0xa");
    builder.add_alias("b", "0xb");
    builder.add_source("dup_a", "module a::dup { public fun f(): u64 { 1 } }");
    builder.add_source("dup_b", "module b::dup { public fun f(): u64 { 2 } }");
    let pkg = builder
        .write_to_temp()
        .expect("failed to create temp package");
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;

    // Ambiguous: two modules named `dup` at different addresses.
    let ambiguous = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({
            "package_path": dir,
            "query": "function_usage",
            "function": "dup::f"
        }),
    )
    .await;
    let mut out = String::from("--- ambiguous ---\n");
    out.push_str(&common::format_tool_result(&ambiguous));

    // Fully-qualified: 3-part addr::module::fun disambiguates to 0xa::dup::f.
    let qualified = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({
            "package_path": dir,
            "query": "function_usage",
            "function": "0xa::dup::f"
        }),
    )
    .await;
    out.push_str("--- qualified ---\n");
    out.push_str(&common::format_tool_result(&qualified));

    common::check_baseline(file!(), &out);
}
