/// Secret redaction for tool output.
///
/// When `security.redact_secrets` is enabled, tool result text is passed
/// through `redact_output()` before being returned to the caller.  The
/// function scans for common secret patterns and replaces matches with
/// `[REDACTED]`.  This is intentionally conservative — it targets
/// high-confidence patterns to minimise false positives that would corrupt
/// patch diffs or structured data.
use regex::Regex;

/// Scan `text` for known secret patterns and replace each match with
/// `[REDACTED]`.
pub fn redact_output(text: &str) -> String {
    let mut out = text.to_string();

    // Bearer / token headers: Authorization: Bearer <token>
    replace(&mut out, &Regex::new(r"(?i)Bearer [A-Za-z0-9\-._~+/]+=*").unwrap());

    // OpenAI-style API keys: sk-<20+ alphanumeric>
    replace(&mut out, &Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap());

    // Generic key- prefix API keys: key-<20+ alphanumeric>
    replace(&mut out, &Regex::new(r"(?i)key-[a-zA-Z0-9]{20,}").unwrap());

    // AWS access key IDs: AKIA + 16 uppercase alphanumerics
    replace(&mut out, &Regex::new(r"AKIA[A-Z0-9]{16}").unwrap());

    // PEM private key blocks (RSA / EC / DSA / OPENSSH)
    replace(
        &mut out,
        &Regex::new(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----")
            .unwrap(),
    );

    // Generic password embedded in URL: ://<user>:<password>@
    replace_password_in_url(&mut out);

    out
}

/// Apply a compiled regex, replacing every match with `[REDACTED]`.
fn replace(text: &mut String, re: &Regex) {
    if re.is_match(text) {
        *text = re.replace_all(text, "[REDACTED]").into_owned();
    }
}

/// Redact passwords embedded in URLs: `://user:secret@host` → `://user:[REDACTED]@host`.
fn replace_password_in_url(text: &mut String) {
    let re = Regex::new(r"://([^:@\s]+):([^@\s]+)@").unwrap();
    if re.is_match(text) {
        *text = re.replace_all(text, "://$1:[REDACTED]@").into_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_bearer_token() {
        let input = "Authorization: Bearer abc123def456==";
        assert_eq!(redact_output(input), "Authorization: [REDACTED]");
    }

    #[test]
    fn redacts_openai_key() {
        let input = "key is sk-abcdefghijklmnopqrstuvwxyz";
        assert_eq!(redact_output(input), "key is [REDACTED]");
    }

    #[test]
    fn redacts_generic_key() {
        let input = "key-abcdefghijklmnopqrstuvwxyz";
        assert_eq!(redact_output(input), "[REDACTED]");
    }

    #[test]
    fn redacts_aws_key() {
        let input = "AKIAIOSFODNN7EXAMPLE";
        assert_eq!(redact_output(input), "[REDACTED]");
    }

    #[test]
    fn redacts_private_key() {
        let input =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        assert!(redact_output(input).contains("[REDACTED]"));
        assert!(!redact_output(input).contains("MIIEpAIBAAKCAQEA"));
    }

    #[test]
    fn redacts_password_in_url() {
        let input = "https://user:mysecretpass@example.com/path";
        assert_eq!(redact_output(input), "https://user:[REDACTED]@example.com/path");
    }

    #[test]
    fn no_redaction_when_clean() {
        let input = "Hello, world! This is a normal string.";
        assert_eq!(redact_output(input), input);
    }
}
