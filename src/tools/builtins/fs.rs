use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct FsTools {
    workspace_root: PathBuf,
}

impl FsTools {
    pub fn new(workspace_root: PathBuf) -> Self {
        FsTools { workspace_root }
    }

    /// 解析路径，确保在 workspace 内
    fn resolve_path(&self, path: &str) -> Result<PathBuf> {
        // 移除前导斜杠，避免绝对路径
        let clean_path = path.trim_start_matches('/');
        
        let full = self.workspace_root.join(clean_path);
        
        // 规范化路径并检查是否超出 workspace 范围
        let canonical = full.canonicalize().unwrap_or_else(|_| full.clone());
        
        if canonical.starts_with(&self.workspace_root) || full.starts_with(&self.workspace_root) {
            Ok(full)
        } else {
            Err(anyhow::anyhow!("路径超出 workspace 范围：{}", path))
        }
    }

    pub fn read(&self, path: &str) -> Result<String> {
        let full_path = self.resolve_path(path)?;
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("读取文件失败：{}", path))?;
        Ok(content)
    }

    pub fn write(&self, path: &str, content: &str) -> Result<String> {
        let full_path = self.resolve_path(path)?;
        
        // 确保父目录存在
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&full_path, content)
            .with_context(|| format!("写入文件失败：{}", path))?;
        Ok(format!("文件已写入：{}", path))
    }

    pub fn patch(&self, path: &str, old_string: &str, new_string: &str) -> Result<String> {
        let full_path = self.resolve_path(path)?;
        
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("读取文件失败：{}", path))?;

        // 查找 old_string 的位置
        let match_count = content.matches(old_string).count();

        if match_count == 0 {
            return Err(anyhow::anyhow!("未找到要替换的内容：{}", old_string));
        }

        if match_count > 1 {
            return Err(anyhow::anyhow!(
                "内容出现 {} 次，无法确定替换位置：{}",
                match_count,
                old_string
            ));
        }

        let new_content = content.replacen(old_string, new_string, 1);

        fs::write(&full_path, &new_content)
            .with_context(|| format!("写入文件失败：{}", path))?;

        Ok(format!("文件已更新：{}", path))
    }

    pub fn list(&self, path: &str) -> Result<String> {
        let full_path = self.resolve_path(path)?;
        
        let dir_path = Path::new(&full_path);

        if !dir_path.exists() {
            return Err(anyhow::anyhow!("目录不存在：{}", path));
        }

        if !dir_path.is_dir() {
            return Err(anyhow::anyhow!("不是目录：{}", path));
        }

        let entries = fs::read_dir(&full_path)
            .with_context(|| format!("读取目录失败：{}", path))?;

        let mut items = Vec::new();
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.path().is_dir();
            items.push(if is_dir {
                format!("📁 {}", name)
            } else {
                format!("📄 {}", name)
            });
        }

        items.sort();
        Ok(items.join("\n"))
    }
}
