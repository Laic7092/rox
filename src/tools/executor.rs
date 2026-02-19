use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::Tool;

use super::builtins::{fs::FsTools, get_time, web};

/// 工具执行器 - 直接持有 FsTools，避免不必要的抽象层
pub struct ToolExecutor {
    fs_tools: FsTools,
}

impl ToolExecutor {
    pub fn new(workspace_root: PathBuf) -> Self {
        ToolExecutor {
            fs_tools: FsTools::new(workspace_root),
        }
    }

    /// 获取所有工具定义
    pub fn get_tools(&self) -> &[Tool] {
        // 引用 registry 中定义的静态工具列表
        use super::registry::get_tools_static;
        get_tools_static()
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
