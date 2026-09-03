---
title: Macros & Meal Plans Design Standards
version: 0.1
---

> This document is a work-in-progress and may change rapidly while the app is under development.

## Guiding Philosophy

### 1. The app shall **never** overwhelm the user

Don't overwhelm the user with information, options or controls.
Present exactly what they need to know to do the job they are there for.

### 2. Show don't tell

Don't over-explain.
Your feature/implementation should not need a wall of text in the app to understand it, if it does then it's either designed incorrectly or the text is superfluous (this seems to be particularly problematic with Claude Code which likes to document every little thing).

### 3. Keep language simple and short

A user should not need a wall of text to understand how to use a field or what they are seeing.
Nor should they need a dictionary to understand the words presented to them.
Keep the language simple and short, it should be understandable by the average person without insulting their indigence.

```markdown
"Items to procure on next retail visit" - ❌ Bad
"Shopping List" - ✅ Good

"Leave empty if this product does not stand in for a generic ingredient." - ❌ Bad
"Optional" - ✅ Good
```

## Conventions

### Units & Numbers

We conform to the International System of Units (SI) standards.

```markdown
100kg - ❌ Bad
100 kg - ✅ Good

100 fluid ounces - ❌ Bad
100 fl oz - ✅ Good
```
