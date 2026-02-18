use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::Tool;

use super::registry::ToolRegistry;

pub struct ToolExecutor {
    registry: ToolRegistry,
}

impl ToolExecutor {
    pub fn new(workspace_root: PathBuf) -> Self {
        ToolExecutor {
            registry: ToolRegistry::new(workspace_root),
        }
    }

    pub fn get_tools(&self) -> Vec<Tool> {
        ToolRegistry::get_tools()
    }

    pub async fn execute(&self, name: &str, args: &HashMap<String, Value>) -> Result<String> {
        self.registry.execute(name, args).await
    }
}
