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

/// Reject duplicate provider names, empty operation sets, unbounded identifiers,
/// and any operation that is not an explicit read-only accessor.
#[must_use]
pub fn provider_catalog_is_read_only() -> bool {
    PROVIDER_OPERATIONS.len() == 8
        && PROVIDER_OPERATIONS
            .iter()
            .enumerate()
            .all(|(index, (provider, operations))| {
                !provider.is_empty()
                    && provider.len() <= 64
                    && provider
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
                    && !PROVIDER_OPERATIONS[..index]
                        .iter()
                        .any(|(name, _)| name == provider)
                    && !operations.is_empty()
                    && operations
                        .iter()
                        .enumerate()
                        .all(|(operation_index, operation)| {
                            operation.starts_with("read_")
                                && operation.len() <= 64
                                && operation
                                    .bytes()
                                    .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
                                && !operations[..operation_index].contains(operation)
                        })
            })
}

/// Returns the exact organization, repository, service, and Zed dependency identity.
#[must_use]
pub fn org_spec() -> OrgSpec {
    assert!(
        provider_catalog_is_read_only(),
        "provider operation catalog must remain read-only"
    );
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
        assert!(provider_catalog_is_read_only());
        assert!(PROVIDER_OPERATIONS.iter().all(|(provider, operations)| {
            !provider.contains('*')
                && operations.len() == 2
                && operations
                    .iter()
                    .all(|operation| operation.starts_with("read_"))
        }));
    }
}
