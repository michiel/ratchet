//! MCP capability definitions and management

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub use super::messages::{
    ClientCapabilities, LoggingCapability, PromptsCapability, ResourcesCapability,
    SamplingCapability, ServerCapabilities, ToolsCapability,
};

/// MCP capabilities container
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpCapabilities {
    /// Client capabilities (if this is a client)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<ClientCapabilities>,

    /// Server capabilities (if this is a server)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerCapabilities>,
}

impl McpCapabilities {
    /// Create capabilities for a client
    pub fn client() -> Self {
        Self {
            client: Some(ClientCapabilities::default()),
            server: None,
        }
    }

    /// Create capabilities for a server
    pub fn server() -> Self {
        Self {
            client: None,
            server: Some(ServerCapabilities::default()),
        }
    }

    /// Create capabilities for both client and server
    pub fn both() -> Self {
        Self {
            client: Some(ClientCapabilities::default()),
            server: Some(ServerCapabilities::default()),
        }
    }

    /// Check if this has client capabilities
    pub fn has_client(&self) -> bool {
        self.client.is_some()
    }

    /// Check if this has server capabilities
    pub fn has_server(&self) -> bool {
        self.server.is_some()
    }
}

impl Default for ClientCapabilities {
    fn default() -> Self {
        Self {
            experimental: HashMap::new(),
            sampling: Some(SamplingCapability {}),
        }
    }
}

impl ClientCapabilities {
    /// Create new client capabilities
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable sampling capability
    pub fn with_sampling(mut self) -> Self {
        self.sampling = Some(SamplingCapability {});
        self
    }

    /// Disable sampling capability
    pub fn without_sampling(mut self) -> Self {
        self.sampling = None;
        self
    }

    /// Add experimental capability
    pub fn with_experimental(mut self, name: impl Into<String>, value: Value) -> Self {
        self.experimental.insert(name.into(), value);
        self
    }

    /// Check if sampling is supported
    pub fn supports_sampling(&self) -> bool {
        self.sampling.is_some()
    }

    /// Check if experimental capability is supported
    pub fn supports_experimental(&self, name: &str) -> bool {
        self.experimental.contains_key(name)
    }
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self {
            experimental: HashMap::new(),
            logging: Some(LoggingCapability {}),
            prompts: Some(PromptsCapability {
                list_changed: false,
            }),
            resources: Some(ResourcesCapability {
                subscribe: false,
                list_changed: false,
            }),
            tools: Some(ToolsCapability {
                list_changed: false,
            }),
            batch: Some(crate::protocol::BatchCapability {
                max_batch_size: 50,
                max_parallel: 5,
                supports_dependencies: true,
                supports_progress: true,
                supported_execution_modes: vec![
                    crate::protocol::BatchExecutionMode::Parallel,
                    crate::protocol::BatchExecutionMode::Sequential,
                    crate::protocol::BatchExecutionMode::Dependency,
                ],
            }),
        }
    }
}

impl ServerCapabilities {
    /// Create new server capabilities
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable logging capability
    pub fn with_logging(mut self) -> Self {
        self.logging = Some(LoggingCapability {});
        self
    }

    /// Disable logging capability
    pub fn without_logging(mut self) -> Self {
        self.logging = None;
        self
    }

    /// Enable prompts capability
    pub fn with_prompts(mut self, list_changed: bool) -> Self {
        self.prompts = Some(PromptsCapability { list_changed });
        self
    }

    /// Disable prompts capability
    pub fn without_prompts(mut self) -> Self {
        self.prompts = None;
        self
    }

    /// Enable resources capability
    pub fn with_resources(mut self, subscribe: bool, list_changed: bool) -> Self {
        self.resources = Some(ResourcesCapability {
            subscribe,
            list_changed,
        });
        self
    }

    /// Disable resources capability
    pub fn without_resources(mut self) -> Self {
        self.resources = None;
        self
    }

    /// Enable tools capability
    pub fn with_tools(mut self, list_changed: bool) -> Self {
        self.tools = Some(ToolsCapability { list_changed });
        self
    }

    /// Disable tools capability
    pub fn without_tools(mut self) -> Self {
        self.tools = None;
        self
    }

    /// Add experimental capability
    pub fn with_experimental(mut self, name: impl Into<String>, value: Value) -> Self {
        self.experimental.insert(name.into(), value);
        self
    }

    /// Check if logging is supported
    pub fn supports_logging(&self) -> bool {
        self.logging.is_some()
    }

    /// Check if prompts are supported
    pub fn supports_prompts(&self) -> bool {
        self.prompts.is_some()
    }

    /// Check if prompts list_changed notifications are supported
    pub fn supports_prompts_list_changed(&self) -> bool {
        self.prompts.as_ref().is_some_and(|p| p.list_changed)
    }

    /// Check if resources are supported
    pub fn supports_resources(&self) -> bool {
        self.resources.is_some()
    }

    /// Check if resource subscription is supported
    pub fn supports_resources_subscribe(&self) -> bool {
        self.resources.as_ref().is_some_and(|r| r.subscribe)
    }

    /// Check if resource list_changed notifications are supported
    pub fn supports_resources_list_changed(&self) -> bool {
        self.resources.as_ref().is_some_and(|r| r.list_changed)
    }

    /// Check if tools are supported
    pub fn supports_tools(&self) -> bool {
        self.tools.is_some()
    }

    /// Check if tools list_changed notifications are supported
    pub fn supports_tools_list_changed(&self) -> bool {
        self.tools.as_ref().is_some_and(|t| t.list_changed)
    }

    /// Check if experimental capability is supported
    pub fn supports_experimental(&self, name: &str) -> bool {
        self.experimental.contains_key(name)
    }
}

/// Capability negotiation helper
#[derive(Debug, Clone)]
pub struct CapabilityNegotiator {
    /// Our capabilities
    local_capabilities: McpCapabilities,

    /// Remote capabilities (if known)
    remote_capabilities: Option<McpCapabilities>,
}

impl CapabilityNegotiator {
    /// Create a new capability negotiator
    pub fn new(local_capabilities: McpCapabilities) -> Self {
        Self {
            local_capabilities,
            remote_capabilities: None,
        }
    }

    /// Set remote capabilities after negotiation
    pub fn set_remote_capabilities(&mut self, remote: McpCapabilities) {
        self.remote_capabilities = Some(remote);
    }

    /// Check if both sides support a client capability
    pub fn supports_client_capability(&self, check: impl Fn(&ClientCapabilities) -> bool) -> bool {
        match (&self.local_capabilities.client, &self.remote_capabilities) {
            (Some(local), Some(remote)) => {
                if let Some(remote_client) = &remote.client {
                    check(local) && check(remote_client)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if both sides support a server capability
    pub fn supports_server_capability(&self, check: impl Fn(&ServerCapabilities) -> bool) -> bool {
        match (&self.local_capabilities.server, &self.remote_capabilities) {
            (Some(local), Some(remote)) => {
                if let Some(remote_server) = &remote.server {
                    check(local) && check(remote_server)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if sampling is mutually supported
    pub fn supports_sampling(&self) -> bool {
        self.supports_client_capability(|caps| caps.supports_sampling())
    }

    /// Check if logging is mutually supported
    pub fn supports_logging(&self) -> bool {
        self.supports_server_capability(|caps| caps.supports_logging())
    }

    /// Check if tools are mutually supported
    pub fn supports_tools(&self) -> bool {
        self.supports_server_capability(|caps| caps.supports_tools())
    }

    /// Check if resources are mutually supported
    pub fn supports_resources(&self) -> bool {
        self.supports_server_capability(|caps| caps.supports_resources())
    }

    /// Check if prompts are mutually supported
    pub fn supports_prompts(&self) -> bool {
        self.supports_server_capability(|caps| caps.supports_prompts())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_client_capabilities() {
        let caps = ClientCapabilities::new()
            .with_sampling()
            .with_experimental("custom_feature", json!({"enabled": true}));

        assert!(caps.supports_sampling());
        assert!(caps.supports_experimental("custom_feature"));
        assert!(!caps.supports_experimental("unknown_feature"));
    }

    #[test]
    fn test_server_capabilities() {
        let caps = ServerCapabilities::new()
            .with_tools(true)
            .with_resources(true, true)
            .without_prompts();

        assert!(caps.supports_tools());
        assert!(caps.supports_tools_list_changed());
        assert!(caps.supports_resources());
        assert!(caps.supports_resources_subscribe());
        assert!(caps.supports_resources_list_changed());
        assert!(!caps.supports_prompts());
    }

    #[test]
    fn test_capability_negotiation() {
        let local = McpCapabilities::both();
        let mut negotiator = CapabilityNegotiator::new(local);

        let remote = McpCapabilities {
            client: Some(ClientCapabilities::new().with_sampling()),
            server: Some(ServerCapabilities::new().with_tools(true)),
        };

        negotiator.set_remote_capabilities(remote);

        assert!(negotiator.supports_sampling());
        assert!(negotiator.supports_tools());
        assert!(negotiator.supports_logging());
    }
}
