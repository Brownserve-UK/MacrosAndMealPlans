# Deferred

This file contains tasks that an LLM has deferred during its work.
Keep entries concise, actionable, and under the app area that owns them. Check completed items
rather than adding implementation notes or validation history.
If an item has been half completed then split the task rather than adding notes on what is already
done. Do not copy the product backlog or restate work that is already captured by the technical
specification or ADRs.

## Nutrition and weight

- [ ] `direction_for()` derives calorie target direction from the nutrient name instead of the
      member's objective, so the current target progress can contradict `NUT-023` and `DEC-069`.
- [ ] Avoid loading every weigh-in in `summary()` when only the latest reading and chart range are
      needed.
- [ ] Let the weigh-in dialog set `recorded_at`; the API and schema already support it.
- [ ] Either add a query for `weightKeys.goal(memberId)` or remove its no-op invalidations.

## Meal planning and consumption

- [ ] The 90-day `REVIEW_LOOKBACK_DAYS` window hides older unresolved meals. Replace it with an
      oldest-unresolved query.
- [ ] Batch the per-entry `records_for_entry` calls in `needs_review`.
- [ ] "Ate something else" currently resolves the member's whole meal. Add per-component
      replacements.
- [ ] Changed outcomes can only change or omit planned foods. Allow them to add a different food.
- [ ] The bodyless not-eaten and reopen routes only act as the signed-in member. Add authorised
      per-member operations for managers.
- [ ] Bulk review only accepts pending outcomes. Add an audited correction flow for resolved
      managed-member and guest outcomes.
- [ ] Redact another member's `MealParticipantView.nutrition` unless the caller has health-data
      access.
- [ ] Add the live nutrition total to `MealEditorDialog`, including recipe nutrition and mixed-unit
      conversion.
- [ ] Populate `get_planner_week` values for `can_opt_out`, `can_join`, and `owner_name` when the web
      client has a consumer for them.
- [ ] `MyPlannerPage` maps `preparation.shortage` into its meal shape but never renders it, so only
      the household planner warns that fewer servings were cooked than people are due.
- [ ] Align the `ensure_slot_free` error with the database-constraint fallback error.
- [ ] Seed an opted-out member with a genuinely free personal slot for manual testing.
- [x] Validate `meal_guest_allocation.allocated_unit` and `confirmed_unit` with `unit_code`, or
      document why those columns must remain unrestricted.
- [ ] Add Consumption Record amendment history so corrections retain the previous nutrition and
      quantity snapshot. Deletion history is retained through soft deletion.

## Recipes and food catalogue

- [ ] Derive recipe nutrition from the products actually drawn from stock instead of averaging all
      candidates.
- [ ] Improve mixed mass and volume handling for ingredient pools. This needs density data or a
      clearer explanation when deduction is indeterminate.
- [ ] Explain what the product-level stock tracking "Default" option resolves to.

## Stock

- [x] `MealPlanService::ensure_prepared` silently records a preparation when a recipe component is
      confirmed and no batch exists, so preparation never has to be recorded explicitly. Remove the
      fallback once it is.

- [x] Raw stock is deducted on the first consumption confirmation as a temporary stand-in. Move the
      trigger to an explicit preparation record.
- [ ] Product stock pages show a "spoken for" breakdown, but ingredient stock pages do not.
- [ ] Replace raw stock-history enum labels such as `mode_changed` with user-facing copy.
- [ ] Protect planned-demand claims from exposing another member's private meal plan to every
      `stock:read` holder.
- [ ] Show pooled ingredients with planned demand but no stock in the default Ingredients view.
- [ ] Surface `DemandGapDto` warnings in the stock UI.
- [ ] Avoid `get_many` plus `list_by_ingredient` on every stock overview request.
- [ ] Add server-side stock search when the list needs pagination.
- [x] `PgPreparedBatchRepository` has no database tests, so its SQL is only exercised by hand.
- [ ] Putting leftovers away moves only the first stock item belonging to a cook, so a batch stored
      in more than one place leaves the rest behind.
- [ ] Remove the late pooled-stock sample-data workaround once the pooled scenario can be seeded
      before historical consumption.
- [x] Collapse the `dish_batch_id` add-then-drop churn in `0001_init.sql` before release, so the
      shipped schema never creates a column it immediately removes.

## Shopping

- [ ] Pending purchases from an unfinished shop can become detached from the focused opportunity,
      leaving no UI from which to resume or cancel them.
- [x] Make finishing a shop atomic across all its purchases.
- [ ] Validate that a purchase product belongs to the requirement's ingredient pool, and remove the
      misleading product-name fallback for invalid API-created purchases.
- [x] Keep product-pinned demand on that product during the shopping coverage walk instead of
      drawing it from anywhere in the pool.
- [ ] Avoid recomputing the full stock and meal-plan snapshot on every `requirements()` call.
- [ ] Reassess `SuggestionReason::UnknownAvailability` if manual items and prediction do not give it
      a useful path.

## Web interface and design

- [ ] Move the hard-coded recipe and stock colours into the theme so dark mode works correctly.
- [ ] Consolidate the repeated `Fact` components where their behaviour matches.
- [ ] Add a shared confirmation dialog for the planner delete flows.
- [ ] Standardise mutation-error state names and alert placement.
- [ ] Review whether `SearchField` and `RouteStates` still justify shared abstractions.
- [ ] Move cross-feature quantity, amount, and date formatters into a shared formatting module, and
      align the stock formatter test filename.
- [ ] Reformat the oversized Instructions line in `RecipeFormPage.tsx` and decide whether formatting
      should be enforced.
- [ ] Add a committed Playwright end-to-end suite covering real queries, authentication, and routing.
- [ ] Settle what kind-chip colour is for. `DESIGN_STANDARDS.md` reserves green for the primary
      action and amber for food needing attention, yet also assigns chip colour by kind, so the
      Recipe chip currently shares green with the primary button.
