use mmp_server::bootstrap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bootstrap::init_tracing();
    let config = bootstrap::load_config()?;
    let pool = bootstrap::connect_and_migrate(&config).await?;
    let catalogue = bootstrap::catalogue_service(&pool);

    let report = bootstrap::apply_seed(&catalogue).await?;
    tracing::info!(
        created = report.created,
        updated = report.updated,
        preserved = report.preserved,
        conflicted = report.conflicted,
        "seed catalogue applied"
    );

    if report.conflicted > 0 {
        tracing::warn!(
            count = report.conflicted,
            "some seed entries were skipped because a local record already uses that name"
        );
    }

    Ok(())
}
