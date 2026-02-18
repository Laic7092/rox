use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    pub base_url: String,
    pub max_iterations: usize,
    pub max_llm_retries: usize,
    pub max_tool_calls: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            model: std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "qwen3:4b-instruct-2507-q4_K_M".to_string()),
            base_url: std::env::var("OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            max_iterations: 10,
            max_llm_retries: 3,
            max_tool_calls: 5,
        }
    }
}

/// Workspace 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub root: PathBuf,
    pub agent_file: PathBuf,
    pub soul_file: PathBuf,
    pub user_file: PathBuf,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        let base = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".brk")
            .join("workspace");
        
        WorkspaceConfig {
            root: base.clone(),
            agent_file: base.join("AGENT.md"),
            soul_file: base.join("SOUL.md"),
            user_file: base.join("USER.md"),
        }
    }
}

/// Session 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub storage_path: PathBuf,
    pub auto_save: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        let base = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".brk")
            .join("sessions");
        
        SessionConfig {
            storage_path: base,
            auto_save: true,
        }
    }
}

/// 统一配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub workspace: WorkspaceConfig,
    pub session: SessionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            agent: AgentConfig::default(),
            workspace: WorkspaceConfig::default(),
            session: SessionConfig::default(),
        }
    }
}

impl Config {
    /// 从文件加载配置
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("解析配置文件失败：{}", path.display()))?;

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// 从默认位置加载配置
    pub fn load_default() -> Result<Self> {
        let path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".brk")
            .join("config.toml");
        
        Self::load(&path)
    }
    
    /// 确保 workspace 目录存在
    pub fn ensure_workspace(&self) -> Result<()> {
        fs::create_dir_all(&self.workspace.root)?;
        Ok(())
    }
    
    /// 确保 sessions 目录存在
    pub fn ensure_sessions(&self) -> Result<()> {
        fs::create_dir_all(&self.session.storage_path)?;
        Ok(())
    }
}
