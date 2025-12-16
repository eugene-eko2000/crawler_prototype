use clap::Parser;

/// CLI arguments, defaultable from environment variables.
#[derive(Debug, Parser)]
pub struct Cli {
    /// Root directory for crawler output.
    #[arg(long, env = "CRAWL_URLS", value_delimiter = ',')]
    pub crawl_urls: Vec<String>,
}
