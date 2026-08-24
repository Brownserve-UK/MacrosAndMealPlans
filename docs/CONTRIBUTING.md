# Contributing

## Database schema

If your changes require modifying the database schema you should write a migration but these should be appended to the current unreleased schema migration rather than writing a new migration file for every change.

```markdown
0001_init.sql (current released schema)
0002_new_schema.sql (next/dev schema) - ✅ Use this one
0003_another_schema.sql (migration you created) - ❌ Bad
```
