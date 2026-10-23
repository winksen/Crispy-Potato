use std::{net::TcpListener, path::Path, process::Command};

#[test]
fn database_commands_classify_unreachable_and_authentication_without_secrets() {
    let base = std::env::var("DATABASE_URL").expect("DATABASE_URL is required for database tests");
    let (_, location) = base
        .rsplit_once('@')
        .expect("test database URL must have credentials");
    let password = "deliberately-wrong-release-test-password";
    let bad_auth = format!("postgres://invalid_release_test_user:{password}@{location}");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let unavailable =
        format!("postgres://private_user:{password}@127.0.0.1:{port}/private_database");
    let commands: &[&[&str]] = &[
        &["migrate"],
        &["sync"],
        &["status"],
        &["ingest", "github"],
        &["quarantine", "list"],
        &["quarantine", "release", "1"],
        &["quarantine", "discard", "1"],
    ];
    for (url, class) in [
        (&unavailable, "database unreachable"),
        (&bad_auth, "database authentication failed"),
    ] {
        for args in commands {
            let output = Command::new(env!("CARGO_BIN_EXE_cli"))
                .args(*args)
                .env("DATABASE_URL", url)
                .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
                .output()
                .unwrap();
            assert!(!output.status.success());
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(text.contains(class), "{args:?}: {text}");
            for private in [
                password,
                url.as_str(),
                "private_user",
                "private_database",
                "invalid_release_test_user",
            ] {
                assert!(
                    !text.contains(private),
                    "{args:?} leaked private connection data"
                );
            }
        }
    }
}
