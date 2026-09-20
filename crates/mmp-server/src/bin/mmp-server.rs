use mmp_server::{app, bootstrap};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bootstrap::init_tracing();
    let config = bootstrap::load_config()?;
    let pool = bootstrap::connect_and_migrate(&config).await?;

    let state = bootstrap::app_state(&config, &pool);

    bootstrap::ensure_bootstrap_user(&state.household, &config.dev_user).await?;

    if config.seed_on_start {
        let report = bootstrap::apply_seed(&state.catalogue).await?;
        tracing::info!(
            created = report.created,
            updated = report.updated,
            preserved = report.preserved,
            conflicted = report.conflicted,
            "seed catalogue applied"
        );
    }

    let (router, _) = app::build(state);
    let router = app::with_web_client(router, config.web_dist.as_deref());

    let listener = TcpListener::bind(config.bind_address).await?;
    tracing::info!(address = %config.bind_address, "listening");
    tracing::info!("api docs at http://{}/docs", config.bind_address);

    axum::serve(listener, router).await?;
    Ok(())
}
