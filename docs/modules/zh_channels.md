# Chinese Platform Channels (zh_channels)

> Integrations for Chinese social and messaging platforms — QQ, Feishu, DingTalk, WeChat, WeCom — and Chinese LLM providers.

## Overview

The zh_channels module provides configuration types and manager structures for Chinese platform integrations. It defines platform configs for QQ, Feishu (Lark), DingTalk, WeChat, and WeCom (WeChat Work), as well as Chinese LLM provider configs for DeepSeek, Doubao, Qwen, Kimi, and Zhipu.

Each platform has a corresponding `ChinesePlatformConfig` with an app ID and secret, and each LLM has a known `base_url` for API access. The `ChinesePlatformManager` acts as a registry for both platforms and LLMs, enabling dynamic registration and listing.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `Platform` | Enum: `QQ`, `Feishu`, `DingTalk`, `WeChat`, `WeCom`. |
| `ChineseLLM` | Enum: `DeepSeek`, `Doubao`, `Qwen`, `Kimi`, `Zhipu`. |
| `ChinesePlatformConfig` | Platform configuration with app ID and secret (secret redacted in Debug). |
| `ChineseLLMConfig` | LLM configuration with API key, optional base URL, and model name. |
| `ChineseMessage` | A message payload from a Chinese platform. |
| `ChinesePlatformManager` | Registry for platforms and LLMs. |

### Relationships

`ChinesePlatformManager` owns two `HashMap<String, T>` registries — one for platforms and one for LLMs — keyed by their canonical name. `ChinesePlatformConfig` redacts its `app_secret` field in `Debug` output for security.

## Public API

### `Platform`

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Platform {
    QQ,
    Feishu,
    DingTalk,
    WeChat,
    WeCom,
}

impl Platform {
    pub fn name(&self) -> &'static str
}
```

- **`name`** — Returns the canonical string name: `"qq"`, `"feishu"`, `"dingtalk"`, `"wechat"`, `"wecom"`.

### `ChineseLLM`

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChineseLLM {
    DeepSeek,
    Doubao,
    Qwen,
    Kimi,
    Zhipu,
}

impl ChineseLLM {
    pub fn name(&self) -> &'static str
    pub fn base_url(&self) -> &'static str
}
```

- **`name`** — Canonical string name: `"deepseek"`, `"doubao"`, `"qwen"`, `"kimi"`, `"zhipu"`.
- **`base_url`** — Known API base URLs:

| Provider | Base URL |
|----------|----------|
| DeepSeek | `https://api.deepseek.com/v1` |
| Doubao | `https://ark.cn-beijing.aliyuncs.com/api/v3` |
| Qwen | `https://dashscope.aliyuncs.com/compatible-mode/v1` |
| Kimi | `https://api.moonshot.cn/v1` |
| Zhipu | `https://open.bigmodel.cn/api/paas/v4` |

### `ChinesePlatformConfig`

```rust
pub struct ChinesePlatformConfig {
    pub platform: Platform,
    pub app_id: String,
    #[serde(skip)]
    pub app_secret: String,
    pub enabled: bool,
}
```

`Debug` is custom-implemented to redact `app_secret` as `[REDACTED]`.

### `ChineseLLMConfig`

```rust
pub struct ChineseLLMConfig {
    pub provider: ChineseLLM,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
}
```

Overrides for `base_url` are optional; defaults come from `ChineseLLM::base_url()`.

### `ChineseMessage`

```rust
pub struct ChineseMessage {
    pub content: String,
    pub sender: Option<String>,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
}
```

Generic message payload from any Chinese platform.

### `ChinesePlatformManager`

```rust
impl ChinesePlatformManager {
    pub fn new() -> Self
    pub fn register_platform(&mut self, config: ChinesePlatformConfig)
    pub fn register_llm(&mut self, config: ChineseLLMConfig)
    pub fn list_platforms(&self) -> Vec<&ChinesePlatformConfig>
    pub fn list_llms(&self) -> Vec<&ChineseLLMConfig>
}
```

- **`new`** — Creates an empty registry.
- **`register_platform`** — Adds or replaces a platform config, keyed by `platform.name()`.
- **`register_llm`** — Adds or replaces an LLM config, keyed by `provider.name()`.
- **`list_platforms`** — Returns references to all registered platform configs.
- **`list_llms`** — Returns references to all registered LLM configs.

## Configuration

Platform and LLM configs are managed programmatically; a `praxis.toml` example:

```toml
[chinese_platforms]
[[chinese_platforms.platforms]]
platform = "QQ"
app_id = "your_app_id"
app_secret = "your_app_secret"
enabled = true

[chinese_llms]
[[chinese_llms.llms]]
provider = "DeepSeek"
api_key = "sk-..."
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"
```

## Usage

```rust
use praxis::zh_channels::{
    ChinesePlatformManager, ChinesePlatformConfig,
    ChineseLLMConfig, ChineseLLM, Platform,
};

let mut manager = ChinesePlatformManager::new();

// Register a platform
manager.register_platform(ChinesePlatformConfig {
    platform: Platform::QQ,
    app_id: "12345".into(),
    app_secret: "secret".into(),
    enabled: true,
});

// Register an LLM
manager.register_llm(ChineseLLMConfig {
    provider: ChineseLLM::DeepSeek,
    api_key: "sk-...".into(),
    base_url: None,
    model: "deepseek-chat".into(),
});

let platforms = manager.list_platforms();
let llms = manager.list_llms();
```

## Data Files

None. All configuration is in-memory and managed via the `ChinesePlatformManager` registry.

## Dependencies

- **`serde` / `serde_json`** — Serialization for config and message types.

## Source

`src/zh_channels.rs`