# Macros & Meal Plans

Macros & Meal Plans is a FOSS calorie tracking and meal planning app.

## Technical specification and ADRs

Before planning or implementing a change, read the current Macros & Meal Plans technical specification and the relevant ADRs in the [Brownserve ADR repository](https://github.com/brownserve-UK/ADRs).

In Brownserve devcontainers, the repository is available at `~/Repositories/Brownserve/ADRs/`. The specification is `02_technical_specifications/2026-08-17-MacrosAndMealPlans.md`; relevant product ADRs are in `01_adrs/` tagged by product in their name/tags.

The specification and ADRs are authoritative. Check the current document status and follow any superseding records. Do not copy their decisions, requirements, terminology, or models into this file, since that would create a second and potentially stale source of truth.

If the requested change conflicts with them, or they do not settle behaviour the change requires, raise that with the user before deciding a direction and if the direction changes substantially capture that in a new ADR.

## Project guidance

- Read `README.md` for the supported development and validation commands.
- Read `docs/CONTRIBUTING.md` before changing database schema or tests.
- Read `docs/DESIGN_STANDARDS.md` before changing user-facing design or copy.
- Until instructed otherwise, add all database migrations to `crates/mmp-postgres/migrations/0001_init.sql`. Do not create additional migration files.
- Keep domain and application behaviour in `crates/mmp-core`, PostgreSQL concerns in `crates/mmp-postgres`, HTTP/authentication/OpenAPI concerns in `crates/mmp-server`, and presentation concerns in `web/`.
- Keep API changes, generated OpenAPI/TypeScript output, migrations, and tests aligned where a change crosses those boundaries.
- Android and iOS code will live in a separate repository, which does not yet exist. Mobile development is out of scope until not having it would create significant problems/pain for the future.
- When adding new features and functionality the `sample-data` build should get seeded data that can be used to test the new functionality (where appropriate)
- When deferring tasks store them in `.ai/deferred.md` so they are not lost between sessions.
- When running inside the `bsdev` devcontainer Playwright is installed globally with the Chromium driver for driving UI tests.
