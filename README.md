# MacrosAndMealPlans

Macros & Meal Plans is a FOSS calorie tracking and meal planning app.

> [!NOTE]
> The app is currently in the early stages of being implemented and is going through rapid changes

## Repository Layout

| Path | Description |
| --- | --- |
| `crates/mmp-core` | Domain model, application services, repository ports. |
| `crates/mmp-postgres` | Database stuff - PostgreSQL adapters and migrations |
| `crates/mmp-server` | Backend logic - Axum transport, generated OpenAPI, auth, binaries |
| `web/` | Frontend stuff - Vite + React + TypeScript + MUI client |

## Running it

> [!NOTE]
> This is temporary to allow us to do quick localdev until a proper build chain has been decided upon.

```sh
docker compose up --build
```

That brings up PostgreSQL, the API and the web client, applies migrations and seeds the catalogue.

| What | Where |
| --- | --- |
| Web client | <http://localhost:5173> |
| API | <http://localhost:7979> |
| API docs | <http://localhost:7979/docs> |

Sign in with `MMP_DEV_USER` / `MMP_DEV_PASSWORD`, which default to `admin` / `changeme`. Copy
`.env.example` to `.env` to change ports or credentials; it is optional, since every value has a
working default.

The Dockerfile avoids BuildKit-only features.

## Testing

> [!NOTE]
> Temporary while we're developing locally and iterating quickly

```sh
docker compose exec app cargo fmt --all -- --check
docker compose exec app cargo clippy --workspace --all-targets --all-features -- -D warnings
docker compose exec app cargo test --workspace

docker compose exec web npm run typecheck
docker compose exec web npm run lint
docker compose exec web npm run test
```

Those need no database. The repository tests are separate because they need a reachable
PostgreSQL; they use `#[sqlx::test]`, which creates a throwaway database per test:

```sh
docker compose exec app cargo test --package mmp-postgres --features db-tests
```

Use `docker compose run --rm --no-deps app <command>` instead if the stack is not already up.

Populate the local database with repeatable sample data for manual testing:

```sh
docker compose run --rm sample-data full
```

Dates default to the current week. Pass a specific Monday when a reproducible period is useful:

```sh
docker compose run --rm sample-data full --week-start 2026-08-24
```

Use `minimal` instead of `full` for a smaller dataset.
