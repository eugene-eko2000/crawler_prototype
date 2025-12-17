use std::{time::{Duration, Instant}};

use anyhow::Result;
use serde_json::Value;
use thirtyfour::prelude::*;
use tokio::time::sleep;

use crate::page_metrics::{Metrics, metrics};

fn progressed(prev: Metrics, curr: Metrics) -> bool {
    let height_up = curr.sh > prev.sh + 50;
    let nodes_up  = curr.nodes > prev.nodes + 25;
    let imgs_up   = curr.imgs > prev.imgs;
    let text_up   = curr.text_len > prev.text_len + 200;

    height_up || nodes_up || imgs_up || text_up
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn keyword_score(label: &str) -> i32 {
    let s = normalize(label);

    // Strong phrases
    let strong = [
        "load more", "show more", "view more", "see more", "more results",
        "next", "older", "continue", "read more",
        "mehr", "weiter", "anzeigen", "laden", "nächste",
        "plus", "suivant", "charger", "mostrar más", "cargar más", "ver más",
    ];

    // Negative phrases (avoid common traps)
    let negative = [
        "accept", "cookie", "consent", "login", "log in", "sign in", "signup", "sign up",
        "subscribe", "newsletter", "share", "close", "cancel", "filter", "sort",
    ];

    let mut score = 0;
    for k in strong {
        if s.contains(k) {
            score += 50;
        }
    }
    for k in negative {
        if s.contains(k) {
            score -= 80;
        }
    }
    score
}

async fn element_label(el: &WebElement) -> Result<String> {
    // Prefer accessible names
    let mut parts = vec![];
    if let Ok(v) = el.attr("aria-label").await { if let Some(v) = v { parts.push(v); } }
    if let Ok(v) = el.attr("title").await { if let Some(v) = v { parts.push(v); } }
    if let Ok(v) = el.attr("value").await { if let Some(v) = v { parts.push(v); } }

    // Fall back to innerText
    if let Ok(t) = el.text().await {
        if !t.trim().is_empty() {
            parts.push(t);
        }
    }

    Ok(parts.join(" | "))
}

async fn is_visible_enabled(el: &WebElement) -> bool {
    el.is_displayed().await.unwrap_or(false) && el.is_enabled().await.unwrap_or(false)
}

async fn do_click(driver: &WebDriver, el: &WebElement) -> Result<bool> {
    let before = metrics(driver).await?;
    // Click (normal click first, JS click fallback)
    let clicked = el.click().await.is_ok();
    if !clicked {
        // JS click fallback
        let _ = driver.execute("arguments[0].click();", vec![el.to_json()?]).await;
    }

    // Wait/poll for change
    for _ in 0..24 { // ~6s total
        sleep(Duration::from_millis(250)).await;
        let after = metrics(driver).await?;
        if progressed(before, after) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Scroll the page until the content is exhausted.
pub async fn scroll_until_stable(
    driver: &WebDriver,
    max_time: Duration,           // total time budget
    max_no_progress_rounds: usize, // stop after N scrolls without new items
) -> Result<bool> {
    let start = Instant::now();
    let mut last = metrics(driver).await?;
    let mut no_progress = 0usize;
    let mut did_progress= false;

    let scroll_args = Vec::<Value>::new();

    while start.elapsed() < max_time {
        // Scroll close to bottom (not exactly bottom to trigger lazy-load earlier)
        driver
            .execute(
                r#"window.scrollTo(0, document.body.scrollHeight - 400);"#,
                scroll_args.clone(),
            )
            .await?;
    
        let mut did_step_progress = false;

        // Some sites need a tiny delay between polls
        sleep(Duration::from_millis(250)).await;

        let curr = metrics(driver).await?;
        if progressed(last, curr) {
            last = curr;
            did_step_progress = true;
        }

        if did_step_progress {
            no_progress = 0;
            did_progress = true;
        } else {
            no_progress += 1;
            last = metrics(driver).await?;
        }

        // If the site is truly done, multiple rounds will show no increase.
        if no_progress >= max_no_progress_rounds {
            break;
        }
    }

    Ok(did_progress)
}

/// Attempt to find and click a "load more"-like control.
/// Returns true if clicking caused the page to "progress" (new data loaded).
pub async fn try_click_load_more(driver: &WebDriver, max_time: Duration) -> Result<bool> {
    // Broad selector for clickable-ish elements
    let candidates = driver.find_all(By::Css(
        r#"button, a[href], [role="button"], input[type="button"], input[type="submit"], [onclick]"#,
    )).await?;

    // Score candidates
    let mut scored: Vec<(i32, WebElement, String)> = Vec::new();
    for el in candidates {
        if !is_visible_enabled(&el).await {
            continue;
        }
        let label = element_label(&el).await?;
        if label.trim().is_empty() {
            continue;
        }

        let mut score = keyword_score(&label);

        // Bonus if class/id hint
        if let Ok(Some(cls)) = el.attr("class").await {
            let c = cls.to_lowercase();
            if c.contains("load") || c.contains("more") || c.contains("pagination") || c.contains("next") {
                score += 15;
            }
        }
        if let Ok(Some(id)) = el.attr("id").await {
            let i = id.to_lowercase();
            if i.contains("load") || i.contains("more") || i.contains("pagination") || i.contains("next") {
                score += 15;
            }
        }

        // Ignore very low scores quickly (keeps it cheap)
        if score >= 20 {
            scored.push((score, el, label));
        }
    }

    // Highest score first
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    // Try top few candidates
    let start_time = Instant::now();
    for (_, el, _) in scored.into_iter().take(5) {
        // Bring into view
        let _ = el.scroll_into_view().await;

        if !do_click(driver, &el).await? {
            continue;
        }

        loop {
            if start_time.elapsed() > max_time {
                break;
            }
            if !do_click(driver, &el).await? {
                break;
            }
        }
        return Ok(true);
    }

    Ok(false)
}