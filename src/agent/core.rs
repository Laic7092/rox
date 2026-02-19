use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;

use crate::config::AgentConfig;
use crate::tools::ToolExecutor;
use crate::types::ToolCall;

use super::context::Context;
use super::llm::LlmClient;
use super::session::SessionManager;

/// Agent - 负责对话循环
pub struct Agent {
    llm_client: LlmClient,
    tool_executor: ToolExecutor,
    config: AgentConfig,
    verbose: bool,
}

impl Agent {
    pub fn new(
        llm_client: LlmClient,
        tool_executor: ToolExecutor,
        config: AgentConfig,
        verbose: bool,
    ) -> Self {
        Agent {
            llm_client,
            tool_executor,
            config,
            verbose,
        }
    }

    /// 对话循环
    pub async fn chat(&mut self, session_manager: &mut SessionManager, user_input: &str) -> Result<String> {
        let ctx = self.current_context_mut(session_manager)
            .ok_or_else(|| anyhow!("没有当前会话"))?;
        ctx.add_user(user_input);

        for _ in 1..=self.config.max_iterations {
            let (messages, tools) = {
                let ctx = self.current_context(session_manager)
                    .ok_or_else(|| anyhow!("没有当前会话"))?;
                (ctx.messages().to_vec(), self.tool_executor.get_tools())
            };

            let response = self.llm_client.chat_with_retry(&messages, Some(tools)).await?;

            let ctx = self.current_context_mut(session_manager).unwrap();

            if let Some(tc) = &response.tool_calls {
                if tc.len() > self.config.max_tool_calls {
                    println!("⚠️ 过多的工具调用 ({}个)\n", tc.len());
                    continue;
                }

                ctx.add_assistant(&response.content, Some(tc.clone()));

                let results = self.execute_tool_calls(tc).await;
                for (id, r) in results {
                    ctx.add_tool_result(&id, &r);
                }
            } else {
                ctx.add_assistant(&response.content, None);
                let _ = session_manager.save_current();
                return Ok(response.content);
            }
        }

        let msg = "对话已达到最大处理次数，请简化问题或重新开始".to_string();
        self.current_context_mut(session_manager).unwrap().add_assistant(&msg, None);
        Ok(msg)
    }

    async fn execute_tool_calls(&self, tool_calls: &[ToolCall]) -> Vec<(String, String)> {
        let mut results = Vec::new();
        for tc in tool_calls {
            let args = self.parse_args(tc).unwrap_or_else(|_| HashMap::new());
            if self.verbose {
                println!("🔧 调用：{}({})", tc.function.name, truncate_args(&args));
            } else {
                println!("🔧 {}", tc.function.name);
            }
            let r = self.tool_executor
                .execute(&tc.function.name, &args)
                .await
                .unwrap_or_else(|e| e.to_string());
            if self.verbose {
                println!("✅ 完成：{}\n", truncate_result(&r));
            }
            results.push((tc.id.clone(), r));
        }
        results
    }

    fn parse_args(&self, tc: &ToolCall) -> Result<HashMap<String, Value>> {
        if tc.function.arguments.is_object() {
            serde_json::from_value(tc.function.arguments.clone())
                .map_err(|e| anyhow!("参数解析失败：{}", e))
        } else {
            serde_json::from_str(tc.function.arguments.as_str().unwrap_or("{}"))
                .map_err(|e| anyhow!("参数解析失败：{}", e))
        }
    }

    fn current_context_mut<'a>(&self, session_manager: &'a mut SessionManager) -> Option<&'a mut Context> {
        session_manager.current_mut().map(|s| s.context_mut())
    }

    fn current_context<'a>(&self, session_manager: &'a SessionManager) -> Option<&'a Context> {
        session_manager.current().map(|s| s.context())
    }
}

fn truncate_args(args: &HashMap<String, Value>) -> String {
    let json = serde_json::to_string(args).unwrap_or_default();
    if json.len() > 80 {
        format!("{}...", &json[..77])
    } else {
        json
    }
}

fn truncate_result(result: &str) -> String {
    if result.len() > 100 {
        format!("{}...", &result[..97])
    } else {
        result.to_string()
    }
}
