//! Chinese Platform Channels — QQ, Feishu, DingTalk, WeChat, WeCom.
//!
//! Integrations for Chinese social/messaging platforms.

use serde::{Deserialize, Serialize};

/// Chinese platform configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChinesePlatformConfig {
    pub platform: Platform,
    pub app_id: String,
    pub app_secret: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Platform {
    QQ,
    Feishu,
    DingTalk,
    WeChat,
    WeCom,
}

impl Platform {
    pub fn name(&self) -> &'static str {
        match self {
            Platform::QQ => "qq",
            Platform::Feishu => "feishu",
            Platform::DingTalk => "dingtalk",
            Platform::WeChat => "wechat",
            Platform::WeCom => "wecom",
        }
    }
}

/// Chinese LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChineseLLMConfig {
    pub provider: ChineseLLM,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChineseLLM {
    DeepSeek,
    Doubao,
    Qwen,
    Kimi,
    Zhipu,
}

impl ChineseLLM {
    pub fn name(&self) -> &'static str {
        match self {
            ChineseLLM::DeepSeek => "deepseek",
            ChineseLLM::Doubao => "doubao",
            ChineseLLM::Qwen => "qwen",
            ChineseLLM::Kimi => "kimi",
            ChineseLLM::Zhipu => "zhipu",
        }
    }

    pub fn base_url(&self) -> &'static str {
        match self {
            ChineseLLM::DeepSeek => "https://api.deepseek.com/v1",
            ChineseLLM::Doubao => "https://ark.cn-beijing.aliyuncs.com/api/v3",
            ChineseLLM::Qwen => "https://dashscope.aliyuncs.com/compatible-mode/v1",
            ChineseLLM::Kimi => "https://api.moonshot.cn/v1",
            ChineseLLM::Zhipu => "https://open.bigmodel.cn/api/paas/v4",
        }
    }
}

/// Chinese platform message payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChineseMessage {
    pub content: String,
    pub sender: Option<String>,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
}

/// Platform manager for Chinese chat integrations.
pub struct ChinesePlatformManager {
    platforms: std::collections::HashMap<String, ChinesePlatformConfig>,
    llms: std::collections::HashMap<String, ChineseLLMConfig>,
}

impl ChinesePlatformManager {
    pub fn new() -> Self {
        Self {
            platforms: std::collections::HashMap::new(),
            llms: std::collections::HashMap::new(),
        }
    }

    pub fn register_platform(&mut self, config: ChinesePlatformConfig) {
        self.platforms.insert(config.platform.name().to_string(), config);
    }

    pub fn register_llm(&mut self, config: ChineseLLMConfig) {
        self.llms.insert(config.provider.name().to_string(), config);
    }

    pub fn list_platforms(&self) -> Vec<&ChinesePlatformConfig> {
        self.platforms.values().collect()
    }

    pub fn list_llms(&self) -> Vec<&ChineseLLMConfig> {
        self.llms.values().collect()
    }
}

impl Default for ChinesePlatformManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_names() {
        assert_eq!(Platform::QQ.name(), "qq");
        assert_eq!(Platform::Feishu.name(), "feishu");
    }

    #[test]
    fn test_llm_base_urls() {
        assert!(ChineseLLM::DeepSeek.base_url().contains("deepseek"));
        assert!(ChineseLLM::Qwen.base_url().contains("dashscope"));
    }
}