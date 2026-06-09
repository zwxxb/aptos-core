// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Smoke test for the `facts` query against the real aptos-framework.
//!
//! This test verifies overall schema shape and the enum-corruption regression guard.
//! The `resource_access` field (which requires deep AST traversal) is not asserted here —
//! see `facts_resource_access` and `facts_access` for targeted coverage of that field.

use crate::tests::common;
use std::path::PathBuf;

/// Query the full aptos-framework and assert the schema is well-formed.
///
/// Regression guard: UserTxnLimitsRequest must appear as kind=enum (3 variants),
/// not as a flat struct with colliding field names.
#[test]
fn move_package_query_facts_aptos_framework_smoke() {
    let pkg_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../framework/aptos-framework");
    let dir = pkg_path.to_str().expect("path is utf-8").to_string();

    // 8 MiB stack: visit_pre_order recurses deeply on large expression trees
    // in debug builds. The MCP server runs on threads with larger stacks in
    // production; this test matches that with an explicit builder.
    let result = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .expect("build runtime");
            rt.block_on(async {
                let client = common::make_client().await;
                common::call_tool(
                    &client,
                    "move_package_query",
                    serde_json::json!({ "package_path": dir, "query": "facts" }),
                )
                .await
            })
        })
        .expect("spawn")
        .join()
        .expect("thread joined");

    assert_ne!(
        result.is_error,
        Some(true),
        "facts query returned error: {}",
        common::format_tool_result(&result)
    );

    let formatted = common::format_tool_result(&result);
    let modules: serde_json::Value =
        serde_json::from_str(&formatted).expect("facts result must be JSON");
    let obj = modules.as_object().expect("top level is an object");

    for required in ["0x1::coin", "0x1::aptos_account", "0x1::transaction_limits"] {
        assert!(
            obj.contains_key(required),
            "facts result missing module {}",
            required
        );
    }

    // Regression guard: friends must be records (`{ module: "address::module" }`)
    // with fully-qualified module names (not bare module names), and at least
    // one real-framework module that declares friends must surface them.
    // `0x1::aptos_account` declares `friend aptos_framework::genesis;` etc.
    let aptos_account = obj
        .get("0x1::aptos_account")
        .expect("0x1::aptos_account present");
    let friends = aptos_account
        .get("friends")
        .and_then(|f| f.as_array())
        .expect("friends is an array");
    assert!(
        !friends.is_empty(),
        "0x1::aptos_account must surface its friend declarations"
    );
    for friend in friends {
        let s = friend
            .get("module")
            .and_then(|m| m.as_str())
            .expect("friend entry has a string `module` field");
        assert!(
            s.starts_with("0x") && s.contains("::"),
            "friend module {:?} must be fully-qualified (address::module)",
            s
        );
    }

    let txn_limits = obj
        .get("0x1::transaction_limits")
        .expect("0x1::transaction_limits present");
    let types = txn_limits
        .get("types")
        .and_then(|t| t.as_array())
        .expect("types is an array");
    let user_req = types
        .iter()
        .find(|t| t.get("name").and_then(|n| n.as_str()) == Some("UserTxnLimitsRequest"))
        .expect("UserTxnLimitsRequest present in 0x1::transaction_limits");
    assert_eq!(
        user_req.get("kind").and_then(|k| k.as_str()),
        Some("enum"),
        "UserTxnLimitsRequest must be kind=enum, not kind=struct"
    );
    let variants = user_req
        .get("variants")
        .and_then(|v| v.as_array())
        .expect("enum has variants array");
    assert!(
        variants.len() >= 3,
        "UserTxnLimitsRequest must have >=3 variants, got {}",
        variants.len()
    );
}
