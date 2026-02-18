use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

use crate::config::{AgentConfig, WorkspaceConfig};
use crate::tools::ToolExecutor;
use crate::types::ToolCall;

use super::context::Context;
use super::llm::LlmClient;

pub struct Agent {
    context: Context,
    llm_client: LlmClient,
    tool_executor: ToolExecutor,
    config: AgentConfig,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        let workspace_config = WorkspaceConfig::default();
        let mut context = Context::new(String::new());
        let _ = context.load_system_prompt(&workspace_config);

        Agent {
            context,
            llm_client: LlmClient::new(config.clone()),
            tool_executor: ToolExecutor::new(workspace_config.root),
            config,
        }
    }

    pub async fn chat(&mut self, user_input: &str) -> Result<String> {
        self.context.add_user(user_input);

        let mut iteration = 0;

        while iteration < self.config.max_iterations {
            iteration += 1;
            println!("🔄 迭代 {}/{}", iteration, self.config.max_iterations);

            let messages = self.context.messages();
            let response = match self.llm_client.chat_with_retry(&messages, Some(&self.tool_executor.get_tools())).await {
                Ok(resp) => resp,
                Err(e) => {
                    let error_msg = format!("抱歉，AI 服务暂时不可用：{}", e);
                    self.context.add_assistant(&error_msg.clone(), None);
                    return Ok(error_msg);
                }
            };

            if let Some(tool_calls) = &response.tool_calls {
                if tool_calls.len() > self.config.max_tool_calls {
                    let warning_msg =
                        format!("检测到过多的工具调用 ({}个)，可能存在问题", tool_calls.len());
                    println!("⚠️ {}", warning_msg);

                    self.context.add_assistant(&warning_msg, None);
                    continue;
                }

                // 先添加 LLM 的 tool_call 响应到上下文
                self.context.add_assistant(&response.content, Some(tool_calls.clone()));

                let tool_results = self.execute_tool_calls(tool_calls).await;

                for (tool_call_id, result) in tool_results {
                    self.context.add_tool_result(&tool_call_id, &result);
                }

                continue;
            } else {
                println!("✅ 获得最终回复");
                return Ok(response.content);
            }
        }

        let timeout_msg = "对话已达到最大处理次数，请简化您的问题或重新开始对话".to_string();
        self.context.add_assistant(&timeout_msg, None);

        Ok(timeout_msg)
    }

    async fn execute_tool_calls(&self, tool_calls: &[ToolCall]) -> Vec<(String, String)> {
        let mut results = Vec::new();

        for tool_call in tool_calls {
            let args: HashMap<String, Value> =
                match if tool_call.function.arguments.is_object() {
                    serde_json::from_value(tool_call.function.arguments.clone())
                } else {
                    let args_str = tool_call.function.arguments.as_str().unwrap_or("{}");
                    serde_json::from_str(args_str)
                } {
                    Ok(args) => args,
                    Err(e) => {
                        let error_msg = format!("工具参数解析失败：{}", e);
                        println!("❌ 工具 {} - {}", tool_call.function.name, error_msg);
                        results.push((tool_call.id.clone(), error_msg));
                        continue;
                    }
                };

            println!("🔧 调用工具：{}({:?})", tool_call.function.name, args);

            let result = match self.tool_executor.execute(&tool_call.function.name, &args).await {
                Ok(res) => {
                    println!("✅ 工具调用成功：{}", res);
                    res
                }
                Err(e) => {
                    let error_msg = format!("工具执行失败：{}", e);
                    println!("❌ {}", error_msg);
                    error_msg
                }
            };

            results.push((tool_call.id.clone(), result));
        }

        results
    }

    /// 获取上下文
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// 清空对话历史
    pub fn clear_history(&mut self) {
        self.context.clear();
    }
}
