use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::AgentConfig;
use crate::types::Message;

use super::context::Context as AgentContext;

/// 会话数据结构（用于序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    id: String,
    system_prompt: String,
    messages: Vec<Message>,
    config: AgentConfig,
    created_at: String,
    updated_at: String,
    name: Option<String>,
}

/// 会话元数据
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
}

/// 会话
pub struct Session {
    id: String,
    context: AgentContext,
    config: AgentConfig,
    metadata: SessionMetadata,
}

impl Session {
    pub fn new(id: String, config: AgentConfig, _workspace_root: &Path) -> Self {
        let mut context = AgentContext::new(String::new());
        // 从 workspace 加载系统提示
        let workspace_config = crate::config::WorkspaceConfig::default();
        let _ = context.load_system_prompt(&workspace_config);
        
        let now = Utc::now();
        Session {
            id,
            context,
            config,
            metadata: SessionMetadata {
                name: None,
                created_at: now,
                updated_at: now,
                message_count: 0,
            },
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn context(&self) -> &AgentContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut AgentContext {
        &mut self.context
    }

    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    pub fn metadata(&self) -> &SessionMetadata {
        &self.metadata
    }

    pub fn rename(&mut self, name: &str) {
        self.metadata.name = Some(name.to_string());
        self.metadata.updated_at = Utc::now();
    }

    /// 保存到文件
    pub fn save(&self, storage_path: &Path) -> Result<()> {
        let data = SessionData {
            id: self.id.clone(),
            system_prompt: self.context.system_prompt().to_string(),
            messages: self.context.raw_messages().to_vec(),
            config: self.config.clone(),
            created_at: self.metadata.created_at.to_rfc3339(),
            updated_at: self.metadata.updated_at.to_rfc3339(),
            name: self.metadata.name.clone(),
        };

        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let path = storage_path.join(format!("{}.json", self.id));
        fs::write(path, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }

    /// 从文件加载
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let data: SessionData = serde_json::from_str(&content)?;

        let mut context = AgentContext::new(data.system_prompt);
        for msg in data.messages {
            context.raw_messages_mut().push(msg);
        }

        let message_count = context.len();

        Ok(Session {
            id: data.id,
            context,
            config: data.config,
            metadata: SessionMetadata {
                name: data.name,
                created_at: DateTime::parse_from_rfc3339(&data.created_at)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&data.updated_at)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                message_count,
            },
        })
    }
}

/// 会话管理器
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    storage_path: PathBuf,
    current_session_id: Option<String>,
}

impl SessionManager {
    pub fn new(storage_path: PathBuf) -> Self {
        SessionManager {
            sessions: HashMap::new(),
            storage_path,
            current_session_id: None,
        }
    }

    /// 创建新会话
    pub fn create(&mut self, name: Option<String>, config: AgentConfig) -> &Session {
        let id = uuid::Uuid::new_v4().to_string();
        let workspace_config = crate::config::WorkspaceConfig::default();
        let mut session = Session::new(id.clone(), config, &workspace_config.root);
        
        if let Some(name) = name {
            session.rename(&name);
        }

        self.sessions.insert(id.clone(), session);
        self.current_session_id = Some(id.clone());
        self.sessions.get(&id).unwrap()
    }

    /// 获取会话
    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    /// 获取可变会话
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// 获取当前会话
    pub fn current(&self) -> Option<&Session> {
        self.current_session_id
            .as_ref()
            .and_then(|id| self.sessions.get(id))
    }

    /// 获取当前可变会话
    pub fn current_mut(&mut self) -> Option<&mut Session> {
        self.current_session_id
            .clone()
            .and_then(|id| self.sessions.get_mut(&id))
    }

    /// 切换会话
    pub fn switch(&mut self, id: &str) -> bool {
        if self.sessions.contains_key(id) {
            self.current_session_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    /// 删除会话
    pub fn delete(&mut self, id: &str) -> bool {
        if self.current_session_id.as_deref() == Some(id) {
            self.current_session_id = None;
        }
        self.sessions.remove(id).is_some()
    }

    /// 列出所有会话
    pub fn list(&self) -> Vec<(&str, &SessionMetadata)> {
        self.sessions
            .iter()
            .map(|(id, session)| (id.as_str(), session.metadata()))
            .collect()
    }

    /// 保存会话
    pub fn save(&self, id: &str) -> Result<()> {
        let session = self.sessions.get(id)
            .ok_or_else(|| anyhow::anyhow!("会话不存在：{}", id))?;
        
        session.save(&self.storage_path)
    }

    /// 保存当前会话
    pub fn save_current(&self) -> Result<()> {
        let id = self.current_session_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("没有当前会话"))?;
        self.save(id)
    }

    /// 加载会话
    pub fn load(&mut self, id: &str) -> Result<()> {
        let path = self.storage_path.join(format!("{}.json", id));
        let session = Session::load(&path)?;
        self.sessions.insert(id.to_string(), session);
        Ok(())
    }

    /// 加载所有会话
    pub fn load_all(&mut self) -> Result<()> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.storage_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                    let _ = self.load(id);
                }
            }
        }

        Ok(())
    }
}
