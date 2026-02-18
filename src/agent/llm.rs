use anyhow::{Context, Result};
use reqwest::Client;

use crate::types::{Message, OllamaRequest, OllamaResponse, Tool};
use crate::config::AgentConfig;

pub struct LlmClient {
    client: Client,
    config: AgentConfig,
}

impl LlmClient {
    pub fn new(config: AgentConfig) -> Self {
        LlmClient {
            client: Client::new(),
            config,
        }
    }

    pub async fn chat_with_retry(&self, messages: &[Message], tools: Option<&[Tool]>) -> Result<Message> {
        let mut last_error = None;

        for attempt in 1..=self.config.max_llm_retries {
            match self.chat(messages, tools).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_llm_retries {
                        println!(
                            "⚠️ LLM 调用失败 (尝试 {}/{})，正在重试...",
                            attempt, self.config.max_llm_retries
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            100 * (1 << attempt),
                        ))
                        .await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "LLM 调用在 {} 次尝试后仍然失败：{:?}",
            self.config.max_llm_retries,
            last_error
        ))
    }

    async fn chat(&self, messages: &[Message], tools: Option<&[Tool]>) -> Result<Message> {
        let request = OllamaRequest {
            model: self.config.model.clone(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            stream: false,
        };

        let url = format!("{}/api/chat", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("调用 Ollama API 失败")?;

        let status = response.status();
        let text = response.text().await.context("读取响应失败")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("Ollama API 错误：{} - {}", status, text));
        }

        let ollama_response: OllamaResponse = serde_json::from_str(&text)
            .with_context(|| format!("解析 Ollama 响应失败，原始内容：{}", text))?;

        if let Some(err) = ollama_response.error {
            return Err(anyhow::anyhow!("Ollama 错误：{}", err));
        }

        Ok(ollama_response.message)
    }
}
