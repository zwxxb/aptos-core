// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `move_package_facts` MCP tool: the single v2 contract surface.

use super::super::{
    common::{mcp_err, tool_error},
    session::{into_structured_call_tool_result, FlowSession},
};
use super::facts::{build_facts, MovePackageFacts};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageFactsParams {
    /// Path to the Move package directory.
    package_path: String,
}

#[tool_router(router = package_facts_router, vis = "pub(crate)")]
impl FlowSession {
    #[tool(
        description = "Produce the full v2 facts payload for a Move package: \
                       canonical module/type/function structure, unified resource \
                       access, a per-site tagged call graph, and compiler \
                       diagnostics. Emitted as MCP structuredContent.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn move_package_facts(
        &self,
        Parameters(params): Parameters<MovePackageFactsParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(self
            .move_package_facts_impl(params)
            .await
            .unwrap_or_else(tool_error))
    }
}

impl FlowSession {
    async fn move_package_facts_impl(
        &self,
        params: MovePackageFactsParams,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!("move_package_facts({})", params.package_path);
        // Strip prefixes against the same canonical path the model compiled
        // from, so emitted file paths are package-relative.
        let key = self.resolve_package_path(&params.package_path);
        let (pkg, _) = self.resolve_package(&params.package_path).await?;
        let data = pkg.lock().map_err(|_| mcp_err("package lock poisoned"))?;
        let facts: MovePackageFacts = build_facts(data.env(), &key);
        log::info!(
            "move_package_facts: {} module(s), {} call edge(s), {} diagnostic(s)",
            facts.modules.len(),
            facts.call_graph.len(),
            facts.diagnostics.len()
        );
        Ok(into_structured_call_tool_result(&facts))
    }
}
