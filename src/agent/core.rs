use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::{AgentConfig, SessionConfig};
use crate::tools::ToolExecutor;
use crate::types::ToolCall;

use super::context::Context;
use super::llm::LlmClient;
use super::session::SessionManager;

pub struct Agent {
    session_manager: SessionManager,
    llm_client: LlmClient,
    tool_executor: ToolExecutor,
    config: AgentConfig,
}

impl Agent {
    pub fn new(config: AgentConfig, session_config: SessionConfig, workspace_root: PathBuf) -> Self {
        let mut session_manager = SessionManager::new(session_config.storage_path);
        
        // 加载所有现有会话
        let _ = session_manager.load_all();
        
        // 如果没有当前会话，创建一个默认的
        if session_manager.current().is_none() {
            session_manager.create(None, config.clone());
        }

        Agent {
            session_manager,
            llm_client: LlmClient::new(config.clone()),
            tool_executor: ToolExecutor::new(workspace_root),
            config,
        }
    }

    /// 获取当前会话的上下文
    fn current_context(&mut self) -> Option<&mut Context> {
        self.session_manager.current_mut().map(|s| s.context_mut())
    }

    /// 获取当前会话 ID
    pub fn current_session_id(&self) -> Option<&str> {
        self.session_manager.current_session_id()
    }

    /// 获取会话管理器（用于 session 命令）
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// 获取可变会话管理器
    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_manager
    }

    pub async fn chat(&mut self, user_input: &str) -> Result<String> {
        // 先添加用户消息
        if let Some(context) = self.current_context() {
            context.add_user(user_input);
        } else {
            return Err(anyhow!("没有当前会话"));
        }

        let max_iterations = self.config.max_iterations;
        let max_tool_calls = self.config.max_tool_calls;

        for iteration in 1..=max_iterations {
            println!("🔄 迭代 {}/{}", iteration, max_iterations);

            // 获取消息和工具
            let (messages, tools) = {
                let context = self.current_context().unwrap();
                (context.messages().to_vec(), self.tool_executor.get_tools())
            };

            let response = match self.llm_client.chat_with_retry(&messages, Some(&tools)).await {
                Ok(resp) => resp,
                Err(e) => {
                    let error_msg = format!("抱歉，AI 服务暂时不可用：{}", e);
                    if let Some(context) = self.current_context() {
                        context.add_assistant(&error_msg.clone(), None);
                    }
                    return Ok(error_msg);
                }
            };

            if let Some(tool_calls) = &response.tool_calls {
                if tool_calls.len() > max_tool_calls {
                    let warning_msg =
                        format!("检测到过多的工具调用 ({}个)，可能存在问题", tool_calls.len());
                    println!("⚠️ {}", warning_msg);

                    if let Some(context) = self.current_context() {
                        context.add_assistant(&warning_msg, None);
                    }
                    continue;
                }

                // 先添加 LLM 的 tool_call 响应到上下文
                if let Some(context) = self.current_context() {
                    context.add_assistant(&response.content, Some(tool_calls.clone()));
                }

                let tool_results = self.execute_tool_calls(tool_calls).await;

                for (tool_call_id, result) in tool_results {
                    if let Some(context) = self.current_context() {
                        context.add_tool_result(&tool_call_id, &result);
                    }
                }

                continue;
            } else {
                println!("✅ 获得最终回复");
                if let Some(context) = self.current_context() {
                    context.add_assistant(&response.content, None);
                }
                // 自动保存当前会话
                let _ = self.save_current_session();
                return Ok(response.content);
            }
        }

        let timeout_msg = "对话已达到最大处理次数，请简化您的问题或重新开始对话".to_string();
        if let Some(context) = self.current_context() {
            context.add_assistant(&timeout_msg, None);
        }

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

    /// 保存当前会话
    pub fn save_current_session(&self) -> Result<()> {
        self.session_manager.save_current()
    }

    /// 清空当前会话历史
    pub fn clear_history(&mut self) {
        if let Some(session) = self.session_manager.current_mut() {
            session.context_mut().clear();
        }
    }
}
