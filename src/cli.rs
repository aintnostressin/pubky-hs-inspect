use clap::{Parser, Subcommand};

/// pubky-hs-inspect — CLI tool for inspecting Pubky homeserver instances
#[derive(Parser, Debug)]
#[command(name = "pubky-hs-inspect")]
#[command(about = "Inspect and gather information about Pubky homeserver instances")]
#[command(version)]
pub struct Cli {
    /// Target homeserver URL or PKRR public key
    #[arg(value_name = "URL")]
    pub url: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Inspect a homeserver — resolve its PKRR, show metadata and user count
    Inspect {
        /// Homeserver public key (z32), domain, or pubky:// URL
        #[arg(value_name = "HOMESERVER")]
        url: String,
    },

    /// Inspect a Pubky user — resolve their homeserver, show storage and endpoints
    InspectUser {
        /// PKRR public key (z32) or pubky:// URL of a user
        #[arg(value_name = "KEY_OR_URL")]
        url: String,
    },

    /// Query PKRR records — show raw _pubky SVCB/HTTPS endpoint data
    Pkdns {
        /// PKRR public key (z32) to query
        #[arg(value_name = "KEY")]
        url: String,
    },

    /// Inspect public storage for a PKRR public key
    Storage {
        /// PKRR public key (z32) or resource URL
        #[arg(value_name = "KEY_OR_URL")]
        url: String,
    },

    /// List files under a path for a user's storage
    Ls {
        /// PKRR public key (z32) or pubky:// URL of a user
        #[arg(value_name = "KEY_OR_URL")]
        url: String,

        /// Path within storage (default: /pub/)
        #[arg(short, long, default_value = "/pub/")]
        path: String,
    },

    /// Show tool version
    Version,

    /// Fetch and print recent file change events from a homeserver
    Events {
        /// Maximum number of events to fetch (optional)
        #[arg(short, long, value_name = "N")]
        limit: Option<u64>,

        /// Fetch events in reverse chronological order
        #[arg(short, long)]
        rev: bool,

        /// Homeserver key (z32), domain, or URL. Defaults to the global URL argument.
        #[arg(value_name = "HOMESERVER")]
        homeserver: Option<String>,
    },
}
