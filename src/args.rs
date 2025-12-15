use clap::Parser;

/// CLI arguments, defaultable from environment variables.
#[derive(Debug, Parser)]
pub struct Cli {
    /// Root directory for crawler output.
    #[arg(env = "CRAWL_URL")]
    pub crawl_url: String,
}
