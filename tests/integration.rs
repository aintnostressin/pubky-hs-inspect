//! Integration tests for pubky-hs-inspect CLI commands.
//!
//! These tests verify CLI parsing, command routing, and output structure.
//! Full integration tests using EphemeralTestnet are marked with #[ignore]
//! and require a working pubky-testnet dependency.

use clap::Parser;

// Import the CLI and commands from the library
use pubky_hs_inspect::cli::{Cli, Commands};
use pubky_hs_inspect::commands;

// ── Test: version command ──────────────────────────────────────────

#[tokio::test]
async fn test_version_command() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "version"]);
    assert!(matches!(cli.command, Some(Commands::Version)));

    let result = commands::run(&cli).await;
    assert!(result.is_ok(), "version command should succeed");
}

// ── Test: CLI parsing - inspect ────────────────────────────────────

#[test]
fn test_parse_inspect_command() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "inspect", "abc123def456"]);
    match cli.command {
        Some(Commands::Inspect { url }) => {
            assert_eq!(url, "abc123def456");
        }
        _ => panic!("expected Inspect command"),
    }
}

// ── Test: CLI parsing - inspect-user ───────────────────────────────

#[test]
fn test_parse_inspect_user_command() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "inspect-user", "xyz789abc012"]);
    match cli.command {
        Some(Commands::InspectUser { url }) => {
            assert_eq!(url, "xyz789abc012");
        }
        _ => panic!("expected InspectUser command"),
    }
}

// ── Test: CLI parsing - pkdns ──────────────────────────────────────

#[test]
fn test_parse_pkdns_command() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "pkdns", "pkrr123key456"]);
    match cli.command {
        Some(Commands::Pkdns { url }) => {
            assert_eq!(url, "pkrr123key456");
        }
        _ => panic!("expected Pkdns command"),
    }
}

// ── Test: CLI parsing - storage ────────────────────────────────────

#[test]
fn test_parse_storage_command() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "storage", "storage123key"]);
    match cli.command {
        Some(Commands::Storage { url }) => {
            assert_eq!(url, "storage123key");
        }
        _ => panic!("expected Storage command"),
    }
}

// ── Test: CLI parsing - ls with default path ───────────────────────

#[test]
fn test_parse_ls_command_default_path() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "ls", "lsuser123key456"]);
    match cli.command {
        Some(Commands::Ls { url, path }) => {
            assert_eq!(url, "lsuser123key456");
            assert_eq!(path, "/pub/");
        }
        _ => panic!("expected Ls command"),
    }
}

// ── Test: CLI parsing - ls with custom path ────────────────────────

#[test]
fn test_parse_ls_command_custom_path() {
    let cli = Cli::parse_from([
        "pubky-hs-inspect",
        "ls",
        "lsuser123key456",
        "--path",
        "/pub/my-app/",
    ]);
    match cli.command {
        Some(Commands::Ls { url, path }) => {
            assert_eq!(url, "lsuser123key456");
            assert_eq!(path, "/pub/my-app/");
        }
        _ => panic!("expected Ls command"),
    }
}

// ── Test: CLI parsing - events with limit ──────────────────────────

#[test]
fn test_parse_events_command() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "events", "-l", "10", "events123key"]);
    match cli.command {
        Some(Commands::Events { homeserver, limit }) => {
            assert_eq!(homeserver, Some("events123key".to_string()));
            assert_eq!(limit, Some(10));
        }
        _ => panic!("expected Events command"),
    }
}

// ── Test: CLI parsing - events with -u flag (positional URL) ──────────

#[test]
fn test_parse_events_command_url_flag() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "https://example.pubky.app", "events"]);
    assert_eq!(cli.url, Some("https://example.pubky.app".to_string()));
    assert!(matches!(cli.command, Some(Commands::Events { .. })));
}

// ── Test: CLI parsing - no subcommand ──────────────────────────────

#[test]
fn test_parse_no_subcommand() {
    let cli = Cli::parse_from(["pubky-hs-inspect"]);
    assert!(cli.command.is_none());
}

// ── Test: Version command output structure ─────────────────────────

#[tokio::test]
async fn test_version_output() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "version"]);
    let result = commands::run(&cli).await;
    assert!(result.is_ok());
    // The version command prints to stdout, which we can't easily capture
    // but we verified it doesn't panic/error above
}

// ── Test: Inspect command with invalid key (tests error handling) ──

#[tokio::test]
async fn test_inspect_with_invalid_key() {
    // Use a key that's too short to test error handling
    let cli = Cli::parse_from(["pubky-hs-inspect", "inspect", "short"]);
    let result = commands::run(&cli).await;
    // Should succeed but print an error message (not panic)
    assert!(
        result.is_ok(),
        "inspect should handle invalid keys gracefully"
    );
}

// ── Test: Inspect-user command with invalid key ────────────────────

#[tokio::test]
async fn test_inspect_user_with_invalid_key() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "inspect-user", "short"]);
    let result = commands::run(&cli).await;
    assert!(
        result.is_ok(),
        "inspect-user should handle invalid keys gracefully"
    );
}

// ── Test: Storage command with invalid key ─────────────────────────

#[tokio::test]
async fn test_storage_with_invalid_key() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "storage", "short"]);
    let result = commands::run(&cli).await;
    assert!(
        result.is_ok(),
        "storage should handle invalid keys gracefully"
    );
}

// ── Test: Ls command with invalid key ──────────────────────────────

#[tokio::test]
async fn test_ls_with_invalid_key() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "ls", "short"]);
    let result = commands::run(&cli).await;
    assert!(result.is_ok(), "ls should handle invalid keys gracefully");
}

// ── Test: Pkdns command with invalid key ───────────────────────────

#[tokio::test]
async fn test_pkdns_with_invalid_key() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "pkdns", "short"]);
    let result = commands::run(&cli).await;
    assert!(
        result.is_ok(),
        "pkdns should handle invalid keys gracefully"
    );
}

// ── Test: Events command without homeserver (tests error handling) ─

#[tokio::test]
async fn test_events_without_homeserver() {
    let cli = Cli::parse_from(["pubky-hs-inspect", "events"]);
    let result = commands::run(&cli).await;
    // Should succeed but print an error message
    assert!(
        result.is_ok(),
        "events should handle missing homeserver gracefully"
    );
}

// ── Integration tests requiring EphemeralTestnet ───────────────────
// These tests use pubky-testnet::EphemeralTestnet to spin up a local
// DHT + homeserver + relay for fully offline testing.
//
// To run these tests:
// 1. Uncomment pubky-testnet in Cargo.toml dev-dependencies
// 2. Ensure the dependency compiles (may require fixing upstream issues)
// 3. Run: cargo test --test integration -- --ignored
//
// Note: These tests are currently disabled due to pubky-testnet
// compilation issues with simple-dns 0.11.2.

// TODO: Uncomment when pubky-testnet dependency is fixed
// #[tokio::test]
// #[ignore = "requires working pubky-testnet dependency"]
// async fn test_inspect_homeserver_integration() { ... }
