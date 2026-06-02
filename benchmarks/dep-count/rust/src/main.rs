#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let resp = reqwest::get("http://example.com").await?;
    println!("status: {}", resp.status());
    Ok(())
}
