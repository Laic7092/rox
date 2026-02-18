use anyhow::Result;
use std::fs;

use crate::config::WorkspaceConfig;
use crate::types::{Message, ToolCall};

/// 上下文 - 管理对话历史和系统提示
pub struct Context {
    system_prompt: String,
    messages: Vec<Message>,
}

impl Context {
    pub fn new(system_prompt: String) -> Self {
        Context {
            system_prompt,
            messages: Vec::new(),
        }
    }

    /// 从 workspace 配置加载系统提示
    pub fn load_system_prompt(&mut self, config: &WorkspaceConfig) -> Result<String> {
        let agent = fs::read_to_string(&config.agent_file)
            .unwrap_or_default();
        let soul = fs::read_to_string(&config.soul_file)
            .unwrap_or_default();
        let user = fs::read_to_string(&config.user_file)
            .unwrap_or_default();

        let mut prompt = String::new();
        
        if !agent.is_empty() {
            prompt.push_str(&format!("## 角色定义\n{}\n\n", agent.trim()));
        }
        if !soul.is_empty() {
            prompt.push_str(&format!("## 对话风格\n{}\n\n", soul.trim()));
        }
        if !user.is_empty() {
            prompt.push_str(&format!("## 用户信息\n{}\n\n", user.trim()));
        }

        // 默认兜底
        if prompt.is_empty() {
            prompt = "你是一个有用的助手。你可以使用工具来帮助用户。\n\
当你调用工具后，请根据工具返回的结果直接回答用户的问题，不要编造信息。\n\
如果工具已经给出了完整答案，请简洁地转述给用户，不要添加多余的自我介绍。".to_string();
        }

        self.system_prompt = prompt.trim().to_string();
        Ok(self.system_prompt.clone())
    }

    /// 添加用户消息
    pub fn add_user(&mut self, content: &str) {
        self.messages.push(Message {
            role: "user".to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    /// 添加助手消息
    pub fn add_assistant(&mut self, content: &str, tool_calls: Option<Vec<ToolCall>>) {
        self.messages.push(Message {
            role: "assistant".to_string(),
            content: content.to_string(),
            tool_calls,
            tool_call_id: None,
        });
    }

    /// 添加工具结果
    pub fn add_tool_result(&mut self, tool_call_id: &str, content: &str) {
        self.messages.push(Message {
            role: "tool".to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
        });
    }

    /// 获取所有消息（包含系统提示）
    pub fn messages(&self) -> Vec<Message> {
        let mut all = Vec::with_capacity(self.messages.len() + 1);
        
        // 添加系统提示
        all.push(Message {
            role: "system".to_string(),
            content: self.system_prompt.clone(),
            tool_calls: None,
            tool_call_id: None,
        });
        
        // 添加对话历史
        all.extend(self.messages.iter().cloned());
        
        all
    }

    /// 获取原始消息（不含系统提示）
    pub fn raw_messages(&self) -> &[Message] {
        &self.messages
    }

    /// 获取原始消息可变引用
    pub fn raw_messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }

    /// 裁剪消息历史，保留最近的 N 条
    pub fn truncate(&mut self, max_messages: usize) {
        if self.messages.len() > max_messages {
            self.messages.drain(0..self.messages.len() - max_messages);
        }
    }

    /// 清空对话历史（保留系统提示）
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// 获取系统提示
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// 消息数量
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
