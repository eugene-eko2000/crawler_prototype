use anyhow::Result;
use thirtyfour::WebDriver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub sh: i64,
    pub nodes: i64,
    pub imgs: i64,
    pub text_len: i64,
}

pub(crate) async fn metrics(driver: &WebDriver) -> Result<Metrics> {
    let v = driver
        .execute(
            r#"
        const sh = document.body ? document.body.scrollHeight : 0;
        const nodes = document.getElementsByTagName('*').length;
        const imgs = document.images ? document.images.length : 0;
        const tl = document.body ? (document.body.innerText || '').length : 0;
        return { sh, nodes, imgs, tl };
        "#,
            Vec::<serde_json::Value>::new(),
        )
        .await?;
    let v = v.json();

    Ok(Metrics {
        sh: v.get("sh").and_then(|x| x.as_i64()).unwrap_or(0),
        nodes: v.get("nodes").and_then(|x| x.as_i64()).unwrap_or(0),
        imgs: v.get("imgs").and_then(|x| x.as_i64()).unwrap_or(0),
        text_len: v.get("tl").and_then(|x| x.as_i64()).unwrap_or(0),
    })
}
