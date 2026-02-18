# brk 项目文档

## 项目概述

**brk** 是一个用 Rust 编写的 CLI Agent，通过 Ollama API 与本地 LLM 交互，采用依赖注入和模块化设计，支持内置工具调用、会话管理和可配置的系统提示。

## 模块架构

```
types (基础类型层)
  ↑
agent/config (配置)
  ↑
agent/context (上下文管理)
  ↑
agent/llm (LLM 通信)
  ↑
tools/registry (工具定义)
  ↑
tools/executor (工具执行器)
  ↑
agent/session (会话管理)
  ↑
agent/core (Agent 核心)
  ↑
cli (用户交互)
  ↑
main (入口)
```

## 模块职责

### `types/` - 基础类型层

| 文件 | 内容 |
|------|------|
| `function.rs` | `Tool`, `FunctionDefinition`, `ToolCall`, `FunctionCall` |
| `ollama.rs` | `Message`, `OllamaRequest`, `OllamaResponse` |

### `agent/` - Agent 核心模块

| 文件 | 职责 |
|------|------|
| `config.rs` | `AgentConfig` - 模型、URL、迭代次数、重试次数等配置 |
| `context.rs` | `Context` - 系统提示加载 + 消息历史管理 |
| `session.rs` | `Session`, `SessionManager` - 会话 CRUD + 持久化 |
| `llm.rs` | `LlmClient` - HTTP 通信、重试逻辑 |
| `core.rs` | `Agent` - 对话流程控制 |

### `tools/` - 工具系统

| 文件 | 职责 |
|------|------|
| `registry.rs` | 工具定义和分发逻辑 |
| `executor.rs` | `ToolExecutor` - 工具执行器 |
| `builtins/fs.rs` | 文件系统工具（read, write, patch, list） |
| `builtins/web.rs` | 网络工具（search, fetch） |
| `builtins/get_time.rs` | 时间工具 |

### `cli.rs` - CLI 交互

用户输入循环、环境配置读取、reedline 集成（UTF-8 支持）

## 工具列表

| 工具 | 功能 | 参数 |
|------|------|------|
| `fs_read` | 读取文件 | `path` |
| `fs_write` | 覆盖写入 | `path`, `content` |
| `fs_patch` | 部分修改（查找替换） | `path`, `old_string`, `new_string` |
| `fs_list` | 列出目录 | `path` |
| `web_search` | 搜索网络（Tavily API） | `query` |
| `web_fetch` | 抓取网页 | `url` |
| `get_time` | 获取当前时间 | - |

## 会话管理

### 设计原则

1. **自动保存** - 每次对话后自动保存，无需手动操作
2. **无感管理** - 默认创建/恢复会话，用户无需关心
3. **简洁命令** - 交互模式使用斜杠命令，CLI 保持语义化

### 交互模式命令

```
/clear        - 清空当前会话历史
/new [名]     - 创建新会话并切换
/quit         - 退出（自动保存）
/help         - 显示帮助
```

### CLI 命令

```bash
brk agent             # 进入交互模式
brk session list      # 列出所有会话（短 ID 显示）
brk session delete <ID>  # 删除会话（支持短 ID 前缀匹配）
```

### SessionManager API

```rust
let mut manager = SessionManager::new(storage_path);

// 创建会话
let session = manager.create(Some("我的会话"), config);

// 获取会话
let session = manager.get("session-id");
let session = manager.get_mut("session-id");

// 列出会话
for (id, metadata) in manager.list() {
    println!("{}: {:?}", id, metadata);
}

// 保存/加载
manager.save("session-id")?;
manager.load("session-id")?;
manager.load_all()?;

// 删除会话
manager.delete("session-id");
```

### 会话持久化

会话以 JSON 格式存储在 `storage_path` 目录下：

```json
{
  "id": "uuid",
  "system_prompt": "...",
  "messages": [...],
  "config": {...},
  "created_at": "2026-02-19T00:00:00Z",
  "updated_at": "2026-02-19T00:00:00Z",
  "name": "我的会话"
}
```

## 系统提示配置

### 配置文件位置

```
~/.brk/
├── AGENT.md    # 角色定义
├── SOUL.md     # 对话风格
└── USER.md     # 用户偏好
```

### 配置文件示例

**AGENT.md**:
```markdown
你是一个专业的编程助手，擅长 Rust、Python 和 Web 开发。
你能够使用工具来帮助用户完成文件操作、网络请求等任务。
```

**SOUL.md**:
```markdown
- 说话简洁明了
- 使用 emoji 增加可读性
- 优先提供代码示例
- 用中文回复
```

**USER.md**:
```markdown
- 用户是 Rust 开发者
- 偏好函数式编程风格
- 正在开发一个 CLI 工具
```

### 合并逻辑

`Context::load_system_prompt()` 会将三个文件合并为：

```markdown
## 角色定义
{AGENT.md 内容}

## 对话风格
{SOUL.md 内容}

## 用户信息
{USER.md 内容}
```

如果配置文件不存在或为空，使用默认提示。

## 构建和运行

### 构建

```bash
cargo build
cargo build --release
```

### 运行

```bash
cargo run
```

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `OLLAMA_MODEL` | `qwen3:4b-instruct-2507-q4_K_M` | Ollama 模型 |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API 地址 |
| `TAVILY_API_KEY` | - | web_search 所需 API 密钥 |

### 测试

```bash
cargo test
```

## 设计模式

### 依赖注入

`Agent` 通过组合注入依赖：
- `LlmClient` - 封装 HTTP 通信
- `ToolExecutor` - 封装工具执行
- `Context` - 封装上下文管理
- `AgentConfig` - 集中配置参数

### 职责分离

- **Context** - 纯内存管理，无 IO
- **Session** - 持久化边界，负责存储

### 模块分层

1. **基础层** (`types/`) - 纯数据结构，无业务逻辑
2. **服务层** (`agent/`, `tools/`) - 业务逻辑，依赖基础层
3. **应用层** (`cli.rs`, `main.rs`) - 用户交互和入口

## 开发约定

- **代码风格**: Rust 官方风格指南
- **错误处理**: `anyhow::Result`
- **异步**: tokio 运行时
- **模块导出**: 通过 `mod.rs` 统一导出公共 API

## 依赖版本

| 依赖 | 版本 |
|------|------|
| reqwest | 0.11 (rustls-tls) |
| tokio | 1.x (full) |
| serde | 1.0 (derive) |
| serde_json | 1.0 |
| anyhow | 1.0 |
| chrono | 0.4 |
| regex | 1.10 |
| uuid | 1.0 (v4) |
| dirs | 5.0 |
| toml | 0.8 |
| reedline | 0.38 |
