fn main() -> anyhow::Result<()> {
    let document = collector_server::openapi();
    println!("{}", serde_json::to_string_pretty(&document)?);
    Ok(())
}
