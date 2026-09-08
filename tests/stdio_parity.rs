use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const FINAL_PROTOCOL: &str = "2025-11-25";
const TOOL_NAMES: &[&str] = &[
    "aws_posture",
    "cloudflare_posture",
    "environment_policy",
    "gcp_posture",
    "github_posture",
    "k8s_posture",
    "nats_posture",
    "neon_posture",
    "org_identity",
    "organization_posture",
    "security_baseline",
    "shared_auth_policy",
    "supabase_posture",
    "telemetry_status",
    "zed_dependency_graph",
];
const PROVIDER_OPERATIONS: &[&str] = &[
    "read_organization",
    "read_latest_workflow_run",
    "read_caller_identity",
    "read_eks_clusters",
    "read_project",
    "read_enabled_services",
    "read_auth_settings",
    "read_data_api_schema",
    "read_projects",
    "read_project_branches",
    "read_zone",
    "read_dns_records",
    "read_deployments",
    "read_pods",
    "read_service_snapshot",
    "read_dependency_snapshot",
];
const PROVIDER_ENV_KEYS: &[&str] = &[
    "ORE_MCP_GITHUB_TOKEN",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "ORE_MCP_AWS_ACCOUNT_ID",
    "ORE_MCP_AWS_EKS_CLUSTERS",
    "ORE_MCP_GCP_PROJECT_ID",
    "ORE_MCP_GCP_PROJECT_NUMBER",
    "ORE_MCP_GCP_ACCESS_TOKEN",
    "ORE_MCP_SUPABASE_URL",
    "ORE_MCP_SUPABASE_SERVICE_ROLE_KEY",
    "ORE_MCP_NEON_ORGANIZATION_ID",
    "ORE_MCP_NEON_PROJECT_ID",
    "ORE_MCP_NEON_API_KEY",
    "ORE_MCP_CLOUDFLARE_ZONE",
    "ORE_MCP_CLOUDFLARE_ZONE_ID",
    "ORE_MCP_CLOUDFLARE_API_TOKEN",
    "ORE_MCP_K8S_ENABLED",
    "ORE_MCP_K8S_NAMESPACE",
    "ORE_MCP_NATS_URL",
];

struct McpProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpProcess {
    fn start() -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cliptown-mcp-server"));
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for key in PROVIDER_ENV_KEYS {
            command.env_remove(key);
        }
        let mut child = command.spawn().expect("spawn cliptown MCP process");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
        Self {
            child,
            stdin: Some(stdin),
            stdout,
            next_id: 1,
        }
    }

    fn request(&mut self, method: &str, params: &Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let frame = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let stdin = self.stdin.as_mut().expect("open child stdin");
        serde_json::to_writer(&mut *stdin, &frame).expect("write request");
        stdin.write_all(b"\n").expect("terminate request");
        stdin.flush().expect("flush request");

        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read response");
        assert!(!line.is_empty(), "MCP process closed before responding");
        assert!(line.len() <= 1024 * 1024, "response exceeded frame bound");
        let response: Value = serde_json::from_str(&line).expect("stdout is JSON-RPC only");
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], id);
        assert!(response.get("error").is_none(), "MCP error: {response}");
        response
    }

    fn notify_initialized(&mut self) {
        let frame = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        let stdin = self.stdin.as_mut().expect("open child stdin");
        serde_json::to_writer(&mut *stdin, &frame).expect("write initialized notification");
        stdin.write_all(b"\n").expect("terminate notification");
        stdin.flush().expect("flush notification");
    }

    fn call_tool(&mut self, name: &str) -> Value {
        let response = self.request("tools/call", &json!({"name": name, "arguments": {}}));
        assert_ne!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true))
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .expect("text tool result");
        assert!(text.len() <= 512 * 1024, "tool result exceeded bound");
        serde_json::from_str(text).expect("tool text is structured JSON")
    }

    fn shutdown(mut self) {
        drop(self.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().expect("poll child") {
                assert!(status.success(), "MCP process failed: {status}");
                break;
            }
            if Instant::now() >= deadline {
                self.child.kill().expect("kill stalled child");
                let _ = self.child.wait();
                panic!("MCP process did not stop after stdin closed");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

#[test]
fn stdio_wire_exposes_org_specific_client_and_provider_parity() {
    let mut mcp = McpProcess::start();
    let initialized = mcp.request(
        "initialize",
        &json!({
            "protocolVersion": FINAL_PROTOCOL,
            "capabilities": {},
            "clientInfo": {"name": "cliptown-parity-test", "version": "1.0.0"},
        }),
    );
    assert_eq!(
        initialized.pointer("/result/protocolVersion"),
        Some(&Value::String(FINAL_PROTOCOL.to_owned()))
    );
    for capability in ["tools", "resources", "prompts"] {
        assert!(initialized
            .pointer(&format!("/result/capabilities/{capability}"))
            .is_some());
    }
    mcp.notify_initialized();

    let tools = mcp.request("tools/list", &json!({}));
    let tool_items = tools
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .expect("tools array");
    let mut names = tool_items
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    names.sort_unstable();
    assert_eq!(names, TOOL_NAMES);
    for tool in tool_items {
        assert_eq!(
            tool.pointer("/inputSchema/additionalProperties"),
            Some(&Value::Bool(false))
        );
        assert_eq!(
            tool.pointer("/annotations/readOnlyHint"),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            tool.pointer("/annotations/destructiveHint"),
            Some(&Value::Bool(false))
        );
        assert_eq!(
            tool.pointer("/annotations/idempotentHint"),
            Some(&Value::Bool(true))
        );
    }

    let resources = mcp.request("resources/list", &json!({}));
    let resources = resources
        .pointer("/result/resources")
        .and_then(Value::as_array)
        .expect("resources array");
    assert_eq!(resources.len(), 3);
    assert!(resources.iter().all(|resource| resource["uri"]
        .as_str()
        .is_some_and(|uri| uri.contains("cliptown"))));

    let prompts = mcp.request("prompts/list", &json!({}));
    assert_eq!(
        prompts
            .pointer("/result/prompts")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(3)
    );

    let identity = mcp.call_tool("org_identity");
    assert_eq!(identity["organization"], "cliptown");
    assert_eq!(identity["repository"], "cliptown/cliptown-mcp-server.rs");
    assert_eq!(identity["protocol"], FINAL_PROTOCOL);
    assert_eq!(identity["clients"].as_array().map(Vec::len), Some(6));

    let posture = mcp.call_tool("organization_posture");
    assert_eq!(posture["organization"], "cliptown");
    assert_eq!(posture["providerCount"], 8);
    assert_eq!(posture["state"], "not_configured");
    assert!(posture["providers"]
        .as_array()
        .is_some_and(|providers| providers.len() == 8
            && providers
                .iter()
                .all(|provider| provider["state"] == "not_configured")));

    let spec_source = include_str!("../src/spec.rs");
    for operation in PROVIDER_OPERATIONS {
        assert!(
            spec_source.contains(operation),
            "missing provider operation {operation}"
        );
    }
    mcp.shutdown();
}
