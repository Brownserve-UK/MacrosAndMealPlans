# Contributing

## Database schema

If your changes require modifying the database schema you should write a migration but these should be appended to the current unreleased schema migration rather than writing a new migration file for every change.

```markdown
0001_init.sql (current released schema)
0002_new_schema.sql (next/dev schema) - ✅ Use this one
0003_another_schema.sql (migration you created) - ❌ Bad
```

## Writing tests

When introducing new features or modifying existing functionality you should consider if tests need to be written.
These should kept in separate `.tests` files named appropriately.

Consider if a test is needed they don't need to exist for every little thing and writing them needlessly causes build times to become inflated, but they should absolute exist for the core logic of the app and in the case of a regression being fixed.

Take care when modifying existing tests they usually exist to protect us from accidental regressions or undesirable outcomes.
LLM's have a habit of rewriting tests to suit their changes instead of questioning why the tests exist in the first place and if their changes go against the desired functionality of the app.
