use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::{FunctionDefinition, Tool};

use super::builtins::{fs::FsTools, get_time, web};

/// 获取静态工具列表
pub fn get_tools_static() -> &'static [Tool] {
    &TOOLS
}

/// 预定义的工具列表（懒加载，只初始化一次）
static TOOLS: Lazy<Vec<Tool>> = Lazy::new(|| {
    vec![
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "fs_read".to_string(),
                description: "读取 workspace 内的文件内容".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于 workspace 的文件路径"
                        }
                    },
                    "required": ["path"]
                }),
            },
        },
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "fs_write".to_string(),
                description: "写入文件到 workspace 内（覆盖模式）".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于 workspace 的文件路径"
                        },
                        "content": {
                            "type": "string",
                            "description": "文件内容"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
        },
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "fs_patch".to_string(),
                description: "部分修改 workspace 内的文件（查找替换）".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于 workspace 的文件路径"
                        },
                        "old_string": {
                            "type": "string",
                            "description": "要查找并替换的内容"
                        },
                        "new_string": {
                            "type": "string",
                            "description": "替换为的新内容"
                        }
                    },
                    "required": ["path", "old_string", "new_string"]
                }),
            },
        },
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "fs_list".to_string(),
                description: "列出 workspace 内的目录内容".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于 workspace 的目录路径"
                        }
                    },
                    "required": ["path"]
                }),
            },
        },
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "web_search".to_string(),
                description: "搜索网络信息".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索关键词"
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "web_fetch".to_string(),
                description: "抓取网页内容".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "网页 URL"
                        }
                    },
                    "required": ["url"]
                }),
            },
        },
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_time".to_string(),
                description: "获取当前时间".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
    ]
});

pub struct ToolRegistry {
    fs_tools: FsTools,
}

impl ToolRegistry {
    pub fn new(workspace_root: PathBuf) -> Self {
        ToolRegistry {
            fs_tools: FsTools::new(workspace_root),
        }
    }

    /// 获取所有工具定义
    pub fn get_tools(&self) -> &[Tool] {
        &TOOLS
    }

    pub async fn execute(&self, name: &str, args: &HashMap<String, Value>) -> Result<String> {
        match name {
            "fs_read" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("缺少 path 参数")?;
                self.fs_tools.read(path)
            }
            "fs_write" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("缺少 path 参数")?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .context("缺少 content 参数")?;
                self.fs_tools.write(path, content)
            }
            "fs_patch" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("缺少 path 参数")?;
                let old_string = args
                    .get("old_string")
                    .and_then(|v| v.as_str())
                    .context("缺少 old_string 参数")?;
                let new_string = args
                    .get("new_string")
                    .and_then(|v| v.as_str())
                    .context("缺少 new_string 参数")?;
                self.fs_tools.patch(path, old_string, new_string)
            }
            "fs_list" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("缺少 path 参数")?;
                self.fs_tools.list(path)
            }
            "web_search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .context("缺少 query 参数")?;
                web::search(query).await
            }
            "web_fetch" => {
                let url = args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .context("缺少 url 参数")?;
                web::fetch(url).await
            }
            "get_time" => Ok(get_time::execute()),
            _ => Err(anyhow::anyhow!("未知工具：{}", name)),
        }
    }
}
