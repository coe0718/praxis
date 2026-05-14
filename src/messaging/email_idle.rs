use crate::bus::MessageBus;
/// Email IMAP IDLE push adapter.
///
/// Connects to an IMAP server and uses the IDLE command to receive
/// real-time push notifications for new emails. Incoming messages are
/// published to the Praxis message bus.
///
/// Requires: `PRAXIS_EMAIL_IMAP_HOST`, `PRAXIS_EMAIL_USERNAME`, `PRAXIS_EMAIL_PASSWORD`
/// Optional: `PRAXIS_EMAIL_IMAP_PORT` (default 993), `PRAXIS_EMAIL_IMAP_MAILBOX` (default INBOX)
use anyhow::{Context, Result};
use std::time::Duration;

/// IMAP IDLE client configuration.
#[derive(Debug, Clone)]
pub struct ImapIdleConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub mailbox: String,
}

impl ImapIdleConfig {
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("PRAXIS_EMAIL_IMAP_HOST")
            .context("PRAXIS_EMAIL_IMAP_HOST is required for IMAP IDLE")?;
        let port: u16 = std::env::var("PRAXIS_EMAIL_IMAP_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(993);
        let username = std::env::var("PRAXIS_EMAIL_USERNAME")
            .context("PRAXIS_EMAIL_USERNAME is required for IMAP IDLE")?;
        let password = std::env::var("PRAXIS_EMAIL_PASSWORD")
            .context("PRAXIS_EMAIL_PASSWORD is required for IMAP IDLE")?;
        let mailbox =
            std::env::var("PRAXIS_EMAIL_IMAP_MAILBOX").unwrap_or_else(|_| "INBOX".to_string());

        Ok(Self {
            host,
            port,
            username,
            password,
            mailbox,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let _ = std::env::var("PRAXIS_EMAIL_IMAP_HOST")
            .context("PRAXIS_EMAIL_IMAP_HOST is required")?;
        let _ =
            std::env::var("PRAXIS_EMAIL_USERNAME").context("PRAXIS_EMAIL_USERNAME is required")?;
        let _ =
            std::env::var("PRAXIS_EMAIL_PASSWORD").context("PRAXIS_EMAIL_PASSWORD is required")?;
        Ok(())
    }
}

/// Run the IMAP IDLE loop. Blocks forever, reconnecting on errors.
///
/// New emails are published to the bus as `message` events with
/// source = "email" and the sender's address as the channel/user ID.
pub fn run_imap_idle(config: ImapIdleConfig, bus: &crate::bus::FileBus) -> Result<()> {
    loop {
        match imap_idle_connect_and_listen(&config, bus) {
            Ok(()) => {
                log::warn!("imap: IDLE session ended, reconnecting in 30s");
            }
            Err(e) => {
                log::error!("imap: IDLE error: {e:#}, reconnecting in 30s");
            }
        }
        std::thread::sleep(Duration::from_secs(30));
    }
}

fn imap_idle_connect_and_listen(config: &ImapIdleConfig, bus: &crate::bus::FileBus) -> Result<()> {
    // Build an IMAP client using the builder API (imap 3.0 alpha).
    let client = imap::ClientBuilder::new(&config.host, config.port)
        .mode(imap::ConnectionMode::Tls)
        .connect()
        .context("failed to connect to IMAP server")?;

    let mut session = client
        .login(&config.username, &config.password)
        .map_err(|e| anyhow::anyhow!("IMAP login failed: {e:?}"))?;

    session.select(&config.mailbox).context("failed to select mailbox")?;

    log::info!(
        "imap: connected to {}:{} — listening for new emails in {}",
        config.host,
        config.port,
        config.mailbox
    );

    // Track the last seen UID to avoid reprocessing.
    let mut last_uid: u32 = 0;

    loop {
        // Enter IDLE mode. The server will push EXISTS responses.
        // wait_while blocks until timeout or callback returns false.
        // Handle is dropped at end of statement, releasing the borrow on session.
        let outcome = {
            let mut idle = session.idle();
            idle.timeout(Duration::from_secs(30));
            idle.wait_while(|_resp| false)
        };

        if let Err(ref e) = outcome {
            log::warn!("imap: IDLE wait error: {e:#}");
            std::thread::sleep(Duration::from_secs(5));
            continue;
        }

        // Search for unseen messages.
        let search_result = session.uid_search("UNSEEN").unwrap_or_default();

        for uid in &search_result {
            if *uid <= last_uid {
                continue;
            }
            last_uid = *uid;

            // Fetch the message envelope for sender info.
            let fetch = session.uid_fetch(uid.to_string(), "ENVELOPE").ok();
            if let Some(ref msgs) = fetch {
                for msg in msgs.iter() {
                    if let Some(envelope) = msg.envelope() {
                        let sender = envelope
                            .from
                            .as_ref()
                            .and_then(|addrs| addrs.first())
                            .map(|addr| {
                                addr.name
                                    .as_ref()
                                    .map(|n| String::from_utf8_lossy(n).to_string())
                                    .or_else(|| {
                                        addr.mailbox
                                            .as_ref()
                                            .map(|m| String::from_utf8_lossy(m).to_string())
                                    })
                                    .unwrap_or_else(|| "unknown".to_string())
                            })
                            .unwrap_or_else(|| "unknown".to_string());

                        let subject = envelope
                            .subject
                            .as_ref()
                            .map(|s| String::from_utf8_lossy(s).to_string())
                            .unwrap_or_else(|| "(no subject)".to_string());

                        let event = crate::bus::BusEvent::new(
                            "message",
                            "email",
                            &sender,
                            &sender,
                            format!("[email] {subject}"),
                        );

                        if let Err(e) = bus.publish(&event) {
                            log::warn!("imap: bus publish failed: {e}");
                        } else {
                            log::info!("imap: new email from {sender}: {subject}");
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imap_config_from_env_missing() {
        unsafe { std::env::remove_var("PRAXIS_EMAIL_IMAP_HOST") };
        unsafe { std::env::remove_var("PRAXIS_EMAIL_USERNAME") };
        unsafe { std::env::remove_var("PRAXIS_EMAIL_PASSWORD") };
        let result = ImapIdleConfig::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn imap_config_validate_missing() {
        unsafe { std::env::remove_var("PRAXIS_EMAIL_IMAP_HOST") };
        let result = ImapIdleConfig::validate_environment();
        assert!(result.is_err());
    }
}
