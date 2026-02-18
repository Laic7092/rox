use anyhow::{Context, Result};
use reqwest::Client;

pub async fn search(query: &str) -> Result<String> {
    // 使用 Tavily 搜索 API
    let api_key = std::env::var("TAVILY_API_KEY")
        .context("TAVILY_API_KEY 环境变量未设置")?;

    let client = Client::new();
    let url = "https://api.tavily.com/search";

    let body = serde_json::json!({
        "api_key": api_key,
        "query": query,
        "search_depth": "basic",
        "include_answer": true
    });

    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .context("发送搜索请求失败")?;

    let status = response.status();
    let text = response.text().await.context("读取响应失败")?;

    if !status.is_success() {
        return Err(anyhow::anyhow!("搜索 API 错误：{} - {}", status, text));
    }

    let result: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("解析搜索结果失败：{}", text))?;

    // 提取搜索结果
    let mut output = String::new();
    
    if let Some(answer) = result.get("answer").and_then(|v| v.as_str()) {
        output.push_str(&format!("摘要：{}\n\n", answer));
    }

    if let Some(results) = result.get("results").and_then(|v| v.as_array()) {
        for (i, item) in results.iter().take(5).enumerate() {
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("无标题");
            let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("无 URL");
            let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");
            
            output.push_str(&format!("{}. {}\n   URL: {}\n   {}\n\n", i + 1, title, url, content));
        }
    }

    if output.is_empty() {
        Ok("未找到相关结果".to_string())
    } else {
        Ok(output)
    }
}

pub async fn fetch(url: &str) -> Result<String> {
    let client = Client::new();

    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (compatible; brk-agent/1.0)")
        .send()
        .await
        .with_context(|| format!("请求 URL 失败：{}", url))?;

    let status = response.status();
    let text = response.text().await.context("读取响应失败")?;

    if !status.is_success() {
        return Err(anyhow::anyhow!("网页请求错误：{} - {}", status, url));
    }

    // 简单清理 HTML，提取文本
    let plain_text = html_to_text(&text);
    
    Ok(plain_text)
}

fn html_to_text(html: &str) -> String {
    // 简单的 HTML 清理
    let mut result = html.to_string();
    
    // 移除 script 和 style 标签
    result = regex::Regex::new(r"(?s)<script[^>]*>.*?</script>")
        .map(|re| re.replace_all(&result, "").to_string())
        .unwrap_or_else(|_| result.clone());
    
    result = regex::Regex::new(r"(?s)<style[^>]*>.*?</style>")
        .map(|re| re.replace_all(&result, "").to_string())
        .unwrap_or_else(|_| result);
    
    // 移除 HTML 标签
    result = regex::Regex::new(r"<[^>]*>")
        .map(|re| re.replace_all(&result, "").to_string())
        .unwrap_or_else(|_| result);
    
    // 解码 HTML 实体
    result = result.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    
    // 清理多余空白
    result = regex::Regex::new(r"\n\s*\n")
        .map(|re| re.replace_all(&result, "\n\n").to_string())
        .unwrap_or_else(|_| result);
    
    result.trim().to_string()
}
