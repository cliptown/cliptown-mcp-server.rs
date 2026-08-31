//! Immutable cliptown identity and provider-operation contract.

use ore_mcp_org_server::OrgSpec;

const DEPENDENCIES: &[&str] = &[
    "ORESoftware/mcp-rust-libs",
    "ores-otel/ores-mcp-server-core-libs.rs",
    "shared-auth/shared-auth-clients",
    "shared-auth/shared-auth-interfaces",
    "shared-auth/shared-auth-lib",
    "zed-pkg/zed-cli",
];

/// Operations inherited from the pinned shared provider implementation.
pub const PROVIDER_OPERATIONS: &[(&str, &[&str])] = &[
    ("github", &["read_organization", "read_latest_workflow_run"]),
    ("aws", &["read_caller_identity", "read_eks_clusters"]),
    ("gcp", &["read_project", "read_enabled_services"]),
    ("supabase", &["read_auth_settings", "read_data_api_schema"]),
    ("neon", &["read_projects", "read_project_branches"]),
    ("cloudflare", &["read_zone", "read_dns_records"]),
    ("k8s_cluster", &["read_deployments", "read_pods"]),
    (
        "nats",
        &["read_service_snapshot", "read_dependency_snapshot"],
    ),
];

/// Returns the exact organization, repository, service, and Zed dependency identity.
#[must_use]
pub fn org_spec() -> OrgSpec {
    debug_assert_eq!(PROVIDER_OPERATIONS.len(), 8);
    OrgSpec {
        organization: "cliptown",
        repository: "cliptown/cliptown-mcp-server.rs",
        service_name: "cliptown-mcp-server",
        package_name: "cliptown-mcp-server",
        dependencies: DEPENDENCIES,
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_and_provider_contract_are_exact() {
        let spec = org_spec();
        assert_eq!(spec.organization, "cliptown");
        assert_eq!(spec.repository, "cliptown/cliptown-mcp-server.rs");
        assert_eq!(PROVIDER_OPERATIONS.len(), 8);
        assert!(PROVIDER_OPERATIONS.iter().all(|(provider, operations)| {
            !provider.contains('*')
                && operations.len() == 2
                && operations
                    .iter()
                    .all(|operation| operation.starts_with("read_"))
        }));
    }
}
