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

    /// Show tool version
    Version,
}
