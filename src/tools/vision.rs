use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::backend::{ContentBlock, ImageUrl, InputContent};
use crate::paths::PraxisPaths;

/// Vision tool that analyzes images using AI vision capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionTool {
    pub name: String,
    pub description: String,
    pub parameters: VisionParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl VisionTool {
    pub fn new() -> Self {
        Self {
            name: "vision_analyze".to_string(),
            description: "Analyze an image using AI vision. Supports URLs and local files."
                .to_string(),
            parameters: VisionParameters {
                image_url: None,
                image_path: None,
                question: None,
                detail: None,
            },
        }
    }

    /// Execute the vision tool with the given parameters.
    pub fn execute(&self, params: &VisionParameters, paths: &PraxisPaths) -> Result<InputContent> {
        let image_url = self.resolve_image_url(params, paths)?;
        let detail = params.detail.clone().unwrap_or_else(|| "auto".to_string());

        let content_blocks = vec![
            ContentBlock::Text {
                text: params
                    .question
                    .clone()
                    .unwrap_or_else(|| "Describe this image in detail.".to_string()),
            },
            ContentBlock::ImageUrl {
                image_url: ImageUrl {
                    url: image_url,
                    detail: Some(detail),
                },
            },
        ];

        Ok(InputContent::Blocks(content_blocks))
    }

    /// Resolve the image URL from either a URL or local file path.
    fn resolve_image_url(&self, params: &VisionParameters, paths: &PraxisPaths) -> Result<String> {
        if let Some(url) = &params.image_url {
            // Validate URL format
            if url.starts_with("http://") || url.starts_with("https://") {
                return Ok(url.clone());
            }
            bail!("Invalid URL format: {url}");
        }

        if let Some(path) = &params.image_path {
            // Resolve relative paths against data directory
            let full_path = if Path::new(path).is_relative() {
                paths.data_dir.join(path)
            } else {
                Path::new(path).to_path_buf()
            };

            // Check if file exists
            if !full_path.exists() {
                bail!("Image file not found: {}", full_path.display());
            }

            // Convert to file URL
            let file_url = format!("file://{}", full_path.display());
            return Ok(file_url);
        }

        bail!("Either image_url or image_path must be provided");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_vision_tool_url() {
        let tool = VisionTool::new();
        let paths = PraxisPaths::for_data_dir(tempdir().unwrap().into_path());
        let params = VisionParameters {
            image_url: Some("https://example.com/image.jpg".to_string()),
            image_path: None,
            question: Some("What is in this image?".to_string()),
            detail: Some("high".to_string()),
        };

        let result = tool.execute(&params, &paths).unwrap();
        match result {
            InputContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 2);
                assert!(
                    matches!(&blocks[0], ContentBlock::Text { text } if text == "What is in this image?")
                );
                assert!(
                    matches!(&blocks[1], ContentBlock::ImageUrl { image_url } if image_url.url == "https://example.com/image.jpg")
                );
            }
            _ => panic!("Expected blocks content"),
        }
    }

    #[test]
    fn test_vision_tool_local_file() {
        let tool = VisionTool::new();
        let temp_dir = tempdir().unwrap();
        let image_path = temp_dir.path().join("test.jpg");
        fs::write(&image_path, "fake image data").unwrap();

        let paths = PraxisPaths::for_data_dir(temp_dir.path().to_path_buf());
        let params = VisionParameters {
            image_url: None,
            image_path: Some("test.jpg".to_string()),
            question: None,
            detail: None,
        };

        let result = tool.execute(&params, &paths).unwrap();
        match result {
            InputContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 2);
                assert!(
                    matches!(&blocks[0], ContentBlock::Text { text } if text == "Describe this image in detail.")
                );
                assert!(
                    matches!(&blocks[1], ContentBlock::ImageUrl { image_url } if image_url.url.starts_with("file://"))
                );
            }
            _ => panic!("Expected blocks content"),
        }
    }

    #[test]
    fn test_vision_tool_missing_file() {
        let tool = VisionTool::new();
        let temp_dir = tempdir().unwrap();
        let paths = PraxisPaths::for_data_dir(temp_dir.path().to_path_buf());
        let params = VisionParameters {
            image_url: None,
            image_path: Some("nonexistent.jpg".to_string()),
            question: None,
            detail: None,
        };

        let result = tool.execute(&params, &paths);
        assert!(result.is_err());
    }
}
