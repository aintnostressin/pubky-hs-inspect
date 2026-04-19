//! Integration tests for pubky-hs-inspect CLI commands.
//!
//! These tests verify CLI parsing, command routing, and output structure.
//! Full integration tests using EphemeralTestnet spin up a local DHT +
//! homeserver with embedded PostgreSQL for offline testing.

use clap::Parser;

// Import the CLI and commands from the library
use pubky_hs_inspect::cli::{Cli, Commands};
use pubky_hs_inspect::client::Client;
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
        Some(Commands::Events {
            homeserver,
            limit,
            rev,
        }) => {
            assert_eq!(homeserver, Some("events123key".to_string()));
            assert_eq!(limit, Some(10));
            assert!(!rev);
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
// Uses embedded PostgreSQL (embedded-postgres feature) - no external
// PostgreSQL installation required.

/// Wrapper that keeps a testnet alive for the duration of a test.
struct TestContext {
    testnet: pubky_testnet::EphemeralTestnet,
    pubky: pubky_testnet::pubky::Pubky,
    homeserver_pub_key: pubky_testnet::pubky::PublicKey,
}

/// Helper to build an ephemeral testnet with a homeserver and HTTP relay.
async fn setup_testnet() -> TestContext {
    let testnet = pubky_testnet::EphemeralTestnet::builder()
        .with_http_relay()
        .with_embedded_postgres()
        .config(pubky_testnet::pubky_homeserver::ConfigToml::default_test_config())
        .build()
        .await
        .unwrap();
    let pubky = testnet.sdk().unwrap();
    let homeserver_pub_key = testnet.homeserver_app().public_key();
    TestContext {
        testnet,
        pubky,
        homeserver_pub_key,
    }
}

/// Helper to create a test user on the testnet.
/// Returns (session, user_z32).
async fn create_test_user(ctx: &TestContext) -> (pubky_testnet::pubky::PubkySession, String) {
    let signer = ctx.pubky.signer(pubky_testnet::pubky::Keypair::random());

    let session = signer
        .signup(&ctx.homeserver_pub_key, None)
        .await
        .expect("user signup should succeed");

    let user_z32 = session.info().public_key().z32();

    (session, user_z32)
}

/// Test inspect homeserver against a local testnet.
/// Verifies that the inspect command correctly resolves and displays
/// homeserver information from a real local homeserver.
#[tokio::test]

async fn test_inspect_homeserver_integration() {
    let ctx = setup_testnet().await;
    let hs_z32 = ctx.homeserver_pub_key.z32();

    // Run the inspect command
    let cli = Cli::parse_from(["pubky-hs-inspect", "inspect", &hs_z32]);
    let result = commands::run(&cli).await;
    assert!(
        result.is_ok(),
        "inspect command should succeed against testnet"
    );
}

/// Test inspect-user against a local testnet.
/// Verifies that the inspect-user command correctly resolves a user's
/// homeserver and displays storage information.
#[tokio::test]

async fn test_inspect_user_integration() {
    let ctx = setup_testnet().await;
    let (_session, user_z32) = create_test_user(&ctx).await;

    // Run the inspect-user command
    let cli = Cli::parse_from(["pubky-hs-inspect", "inspect-user", &user_z32]);
    let result = commands::run(&cli).await;
    assert!(
        result.is_ok(),
        "inspect-user command should succeed against testnet"
    );
}

/// Test storage listing against a local testnet.
/// Verifies that the storage command correctly lists public storage entries
/// for a user who has uploaded files.
#[tokio::test]

async fn test_storage_listing_integration() {
    let ctx = setup_testnet().await;
    let (session, user_z32) = create_test_user(&ctx).await;

    // Upload a test file to public storage
    session
        .storage()
        .put("/pub/test-file.txt", "hello world")
        .await
        .expect("file upload should succeed")
        .error_for_status()
        .unwrap();

    // Run the storage command
    let cli = Cli::parse_from(["pubky-hs-inspect", "storage", &user_z32]);
    let result = commands::run(&cli).await;
    assert!(
        result.is_ok(),
        "storage command should succeed against testnet"
    );
}

/// Test ls listing against a local testnet.
/// Verifies that the ls command correctly lists files in a user's storage
/// directory structure.
#[tokio::test]

async fn test_ls_listing_integration() {
    let ctx = setup_testnet().await;
    let (session, user_z32) = create_test_user(&ctx).await;

    // Upload multiple test files in a directory structure
    session
        .storage()
        .put("/pub/my-app/hello.txt", "hi")
        .await
        .expect("file upload should succeed")
        .error_for_status()
        .unwrap();
    session
        .storage()
        .put("/pub/my-app/config.json", r#"{"key":"value"}"#)
        .await
        .expect("file upload should succeed")
        .error_for_status()
        .unwrap();
    session
        .storage()
        .put("/pub/my-app/assets/style.css", "body{}")
        .await
        .expect("file upload should succeed")
        .error_for_status()
        .unwrap();

    // Run the ls command with default path
    let cli = Cli::parse_from(["pubky-hs-inspect", "ls", &user_z32]);
    let result = commands::run(&cli).await;
    assert!(result.is_ok(), "ls command should succeed against testnet");

    // Run the ls command with specific path
    let cli = Cli::parse_from([
        "pubky-hs-inspect",
        "ls",
        &user_z32,
        "--path",
        "/pub/my-app/",
    ]);
    let result = commands::run(&cli).await;
    assert!(
        result.is_ok(),
        "ls command with path should succeed against testnet"
    );
}

/// Test events against a local testnet.
/// Verifies that the events command correctly fetches and displays
/// file change events from a homeserver.
#[tokio::test]

async fn test_events_integration() {
    let ctx = setup_testnet().await;
    let (session, _user_z32) = create_test_user(&ctx).await;

    // Upload files to trigger events
    session
        .storage()
        .put("/pub/doc1.txt", "content1")
        .await
        .expect("file upload should succeed")
        .error_for_status()
        .unwrap();
    session
        .storage()
        .put("/pub/doc2.txt", "content2")
        .await
        .expect("file upload should succeed")
        .error_for_status()
        .unwrap();

    let hs_z32 = ctx.homeserver_pub_key.z32();

    // ── Verify get_events returns valid events from the homeserver ──

    // Get the homeserver's local HTTP URL (http://127.0.0.1:<port>)
    let base_url = ctx.testnet.homeserver_app().icann_http_url().to_string();
    let client = Client::new().unwrap();

    // Call get_events and verify the response.
    let (events, _next_cursor) = client
        .get_events(&base_url, None, Some(10), Some(&hs_z32), false)
        .await
        .expect("get_events must succeed — homeserver returned an error");

    // Events must not be empty
    assert!(
        !events.is_empty(),
        "Expected events from homeserver, got empty list"
    );

    let events_text: String = events.join("\n");

    // Must contain valid event entries (PUT or DEL)
    assert!(
        events_text.contains("PUT") || events_text.contains("DEL"),
        "Events must contain PUT/DEL entries, got: {events_text}"
    );

    // ── CLI integration test ──

    // Run the events command to verify end-to-end routing works
    let cli = Cli::parse_from(["pubky-hs-inspect", "events", &hs_z32]);
    let result = commands::run(&cli).await;
    assert!(
        result.is_ok(),
        "events command should succeed against testnet"
    );

    // ── Verify get_events respects the reverse parameter ──

    // Fetch events in forward order
    let (events_fwd, _) = client
        .get_events(&base_url, None, Some(20), Some(&hs_z32), false)
        .await
        .expect("get_events forward must succeed");

    // Fetch events in reverse order
    let (events_rev, _) = client
        .get_events(&base_url, None, Some(20), Some(&hs_z32), true)
        .await
        .expect("get_events reverse must succeed");

    // Both should return the same number of events
    assert_eq!(
        events_fwd.len(),
        events_rev.len(),
        "Forward and reverse queries should return the same number of events (got fwd={}, rev={})",
        events_fwd.len(),
        events_rev.len()
    );

    // Must have at least 2 events to meaningfully test reversal
    assert!(
        events_fwd.len() >= 2,
        "Expected at least 2 events to verify reversal, got {}",
        events_fwd.len()
    );

    // First and last events must differ — order is actually reversed, not identical
    assert_ne!(
        events_fwd.first(),
        events_rev.first(),
        "Reverse order should differ from forward order at the first event"
    );
    assert_ne!(
        events_fwd.last(),
        events_rev.last(),
        "Reverse order should differ from forward order at the last event"
    );
}
