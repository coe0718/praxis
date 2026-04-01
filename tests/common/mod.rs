use assert_cmd::Command;

pub fn praxis_command() -> Command {
    Command::cargo_bin("praxis").unwrap()
}
