use std::time::{Duration, Instant};

use anyhow::Result;
use thirtyfour::{ChromiumLikeCapabilities, DesiredCapabilities, WebDriver};
use tokio::time::sleep;
use tracing::info;

use crate::{
    page_metrics::metrics,
    scrolling::{scroll_until_stable, try_click_load_more},
};

/// Wait until page stops changing for `quiet_for`, or until `timeout`.
async fn wait_for_dom_quiet(
    driver: &WebDriver,
    timeout: Duration,
    quiet_for: Duration,
) -> Result<()> {
    let start = Instant::now();
    let mut last = metrics(driver).await?;
    let mut last_change = Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Ok(()); // best-effort: return what we have
        }

        sleep(Duration::from_millis(250)).await;
        let cur = metrics(driver).await?;

        // Use thresholds to ignore tiny churn (timers/counters)
        let changed = cur.sh > last.sh + 20
            || cur.nodes > last.nodes + 10
            || cur.imgs > last.imgs
            || cur.text_len > last.text_len + 50;

        if changed {
            last = cur;
            last_change = Instant::now();
        } else if last_change.elapsed() >= quiet_for {
            return Ok(());
        }
    }
}

pub async fn crawl_page(url: &str) -> Result<String> {
    // Launch browser (headless mode)
    let mut caps = DesiredCapabilities::chrome();
    caps.add_arg("--headless=new")?;
    caps.add_arg("--no-sandbox")?;
    caps.add_arg("--disable-gpu")?;
    caps.add_arg("--window-size=1280,900")?;
    caps.add_arg("--disable-dev-shm-usage")?;
    let driver = WebDriver::new("http://localhost:9515", caps).await?;

    // Open the target page
    driver.goto(url).await?;

    // Wait for DOM to settle (hydration + late async updates)
    wait_for_dom_quiet(&driver, Duration::from_secs(20), Duration::from_secs(2)).await?;

    // Scrolling until we get a new content
    let scroll_paging = scroll_until_stable(&driver, Duration::from_secs(45), 3).await?;
    info!("Scroll paging detected: {scroll_paging}");

    // Trying to scroll with pressing a Load more... button
    let button_paging = try_click_load_more(&driver).await?;
    info!("Button paging detected: {button_paging}");

    let html = driver.source().await?;

    driver.quit().await?;
    Ok(html)
}
