use tempfile::tempdir;

use super::AppConfig;

#[test]
fn saves_and_loads_valid_config() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("praxis.toml");
    let config = AppConfig::default_for_data_dir(temp.path().join("data"));

    config.save(&path).unwrap();
    let loaded = AppConfig::load(&path).unwrap();

    assert_eq!(loaded, config);
}

#[test]
fn accepts_claude_backend() {
    let mut config = AppConfig::default_for_data_dir("/tmp/praxis".into());
    config.agent.backend = "claude".to_string();
    config.agent.model_pin = Some("claude-3-5-sonnet-latest".to_string());

    config.validate().unwrap();
}

#[test]
fn accepts_router_backend() {
    let mut config = AppConfig::default_for_data_dir("/tmp/praxis".into());
    config.agent.backend = "router".to_string();

    config.validate().unwrap();
}

#[test]
fn rejects_invalid_backend() {
    let mut config = AppConfig::default_for_data_dir("/tmp/praxis".into());
    config.agent.backend = "gpt".to_string();

    let error = config.validate().unwrap_err().to_string();
    assert!(error.contains("agent.backend"));
}

#[test]
fn rejects_duplicate_context_priorities() {
    let mut config = AppConfig::default_for_data_dir("/tmp/praxis".into());
    config.context.budget[1].priority = config.context.budget[0].priority;

    let error = config.validate().unwrap_err().to_string();
    assert!(error.contains("priorities"));
}
