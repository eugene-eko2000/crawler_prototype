use std::{
    fs,
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::Result;
use chrome_driver_rs::ensure_latest_driver;
use clap::Parser;
use thirtyfour::{error::WebDriverErrorInfo, prelude::*};
use tokio::time::sleep;
use tracing_subscriber::EnvFilter;

use crate::{crawl::crawl_page, url_decorate::url_to_filename};

mod args;
mod crawl;
mod page_metrics;
mod scrolling;
mod url_decorate;

// const URL: &str = "https://web-scraping.dev/reviews";
// const URL: &str = "https://web-scraping.dev/testimonials";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = args::Cli::parse();

    // Automatically download and use the latest ChromeDriver
    let driver_info = ensure_latest_driver("./driver")
        .await
        .map_err(|e| WebDriverError::UnknownError(WebDriverErrorInfo::new(e.to_string())))?;

    // Start ChromeDriver
    Command::new(&driver_info.driver_path)
        .args(["--port=9515"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start chromedriver");

    sleep(Duration::from_secs(3)).await;

    let html = crawl_page(&cli.crawl_url).await?;

    // Save result
    fs::write(url_to_filename(&cli.crawl_url), html)?;

    Ok(())
}
