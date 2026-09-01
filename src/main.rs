//! Fleet-generated organization MCP server entry point.

mod spec;

use ore_mcp_org_server::run_stdio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_stdio(spec::org_spec()).await
}
