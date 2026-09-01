//! OAuth-protected Streamable HTTP entry point for remote MCP clients.

mod spec;

use ore_mcp_org_server::run_http;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_http(spec::org_spec()).await
}
