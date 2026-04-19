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
    /// Inspect a PKRR public key — resolve homeserver, show endpoints
    Inspect {
        /// PKRR public key (z32) or pubky:// URL
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
