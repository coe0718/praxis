use chrono::TimeZone;
use tempfile::tempdir;

use super::LocalIdentityPolicy;
use crate::{config::AppConfig, identity::IdentityPolicy, paths::PraxisPaths};

#[test]
fn validates_identity_files() {
    let temp = tempdir().unwrap();
    let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
    let config = AppConfig::default_for_data_dir(paths.data_dir.clone());
    let policy = LocalIdentityPolicy;
    let now = chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap();

    policy.ensure_foundation(&paths, &config, now).unwrap();
    policy.validate(&paths).unwrap();
}
