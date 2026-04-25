//! OAuth 2.0 authorization for external service integrations.
//!
//! Praxis uses OAuth to let the agent interact with services on the operator's
//! behalf: reading Gmail, managing calendar events, accessing GitHub repositories,
//! and so on. LLM providers (Claude, OpenAI, Ollama) are authenticated separately
//! via API keys stored in environment variables.
//!
//! # Supported providers
//!
//! | Provider | Flow              | Env vars required                                              |
//! |----------|-------------------|----------------------------------------------------------------|
//! | GitHub   | Device code       | `PRAXIS_GITHUB_OAUTH_CLIENT_ID`                                |
//! | Google   | Device code       | `PRAXIS_GOOGLE_OAUTH_CLIENT_ID`, `PRAXIS_GOOGLE_OAUTH_CLIENT_SECRET` |
//!
//! # Token storage
//!
//! Tokens are stored in `oauth_tokens.json` inside the Praxis data directory.
//! The file is written with `600` permissions (owner-only) on Unix.
//! Never commit this file to version control.
//!
//! # Usage
//!
//! ```text
//! praxis oauth login github
//! praxis oauth login google
//! praxis oauth status
//! praxis oauth token github      # print raw token for scripting
//! praxis oauth refresh google    # force a token refresh
//! praxis oauth revoke github
//! ```

pub mod copilot;
mod device_flow;
pub mod github;
pub mod github_client;
pub mod gmail;
pub mod google;
pub mod store;

pub use copilot::CopilotOAuth;
pub use github::GitHubOAuth;
pub use github_client::GitHubClient;
pub use gmail::GmailClient;
pub use google::GoogleOAuth;
pub use store::{OAuthToken, OAuthTokenStore};
