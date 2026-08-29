CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE ingredient (
    id                  UUID PRIMARY KEY,
    name                TEXT NOT NULL,
    default_unit        TEXT NOT NULL,

    origin              TEXT NOT NULL,
    seed_key            TEXT,
    source_provider     TEXT,
    source_external_id  TEXT,
    locally_modified    BOOLEAN NOT NULL DEFAULT FALSE,

    revision            BIGINT NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at         TIMESTAMPTZ,

    CONSTRAINT ingredient_name_not_blank
        CHECK (btrim(name) <> ''),
    CONSTRAINT ingredient_origin_valid
        CHECK (origin IN ('seeded', 'local', 'external')),
    CONSTRAINT ingredient_default_unit_valid
        CHECK (default_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch')),
    CONSTRAINT ingredient_seeded_has_key
        CHECK (origin <> 'seeded' OR seed_key IS NOT NULL)
);

CREATE UNIQUE INDEX ingredient_name_unique ON ingredient (lower(name));
CREATE UNIQUE INDEX ingredient_seed_key_unique ON ingredient (seed_key) WHERE seed_key IS NOT NULL;
CREATE INDEX ingredient_name_trgm ON ingredient USING GIN (name gin_trgm_ops);

CREATE TABLE product (
    id                       UUID PRIMARY KEY,
    name                     TEXT NOT NULL,
    brand                    TEXT,
    barcode                  TEXT,
    retailer                 TEXT,
    shopping_section         TEXT,
    package_quantity_amount  NUMERIC(16, 4),
    package_quantity_unit    TEXT,
    servings_per_pack        INTEGER,
    mapped_ingredient_id     UUID REFERENCES ingredient (id) ON DELETE RESTRICT,

    nutrition_basis_amount  NUMERIC(16, 4),
    nutrition_basis_unit    TEXT,
    energy_kcal         NUMERIC(12, 3),
    protein_g           NUMERIC(12, 3),
    carbohydrate_g      NUMERIC(12, 3),
    sugar_g             NUMERIC(12, 3),
    fat_g               NUMERIC(12, 3),
    saturated_fat_g     NUMERIC(12, 3),
    fibre_g             NUMERIC(12, 3),
    salt_g              NUMERIC(12, 3),
    cholesterol_mg      NUMERIC(12, 3),
    nutrition_extra     JSONB NOT NULL DEFAULT '{}'::jsonb,

    origin              TEXT NOT NULL,
    seed_key            TEXT,
    source_provider     TEXT,
    source_external_id  TEXT,
    locally_modified    BOOLEAN NOT NULL DEFAULT FALSE,

    revision            BIGINT NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at         TIMESTAMPTZ,

    CONSTRAINT product_name_not_blank
        CHECK (btrim(name) <> ''),
    CONSTRAINT product_origin_valid
        CHECK (origin IN ('seeded', 'local', 'external')),
    CONSTRAINT product_barcode_valid
        CHECK (barcode IS NULL OR barcode ~ '^[0-9]{4,18}$'),
    CONSTRAINT product_package_unit_valid
        CHECK (package_quantity_unit IS NULL OR package_quantity_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch')),
    CONSTRAINT product_package_quantity_complete
        CHECK (num_nonnulls(package_quantity_amount, package_quantity_unit) <> 1),
    CONSTRAINT product_package_quantity_positive
        CHECK (package_quantity_amount IS NULL OR package_quantity_amount > 0),
    CONSTRAINT product_servings_per_pack_positive
        CHECK (servings_per_pack IS NULL OR servings_per_pack > 0),
    CONSTRAINT product_basis_unit_valid
        CHECK (nutrition_basis_unit IS NULL OR nutrition_basis_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch')),
    CONSTRAINT product_basis_complete
        CHECK (num_nonnulls(nutrition_basis_amount, nutrition_basis_unit) <> 1),
    CONSTRAINT product_basis_positive
        CHECK (nutrition_basis_amount IS NULL OR nutrition_basis_amount > 0),
    CONSTRAINT product_extra_is_object
        CHECK (jsonb_typeof(nutrition_extra) = 'object'),
    CONSTRAINT product_energy_kcal_non_negative
        CHECK (energy_kcal IS NULL OR energy_kcal >= 0),
    CONSTRAINT product_protein_g_non_negative
        CHECK (protein_g IS NULL OR protein_g >= 0),
    CONSTRAINT product_carbohydrate_g_non_negative
        CHECK (carbohydrate_g IS NULL OR carbohydrate_g >= 0),
    CONSTRAINT product_sugar_g_non_negative
        CHECK (sugar_g IS NULL OR sugar_g >= 0),
    CONSTRAINT product_fat_g_non_negative
        CHECK (fat_g IS NULL OR fat_g >= 0),
    CONSTRAINT product_saturated_fat_g_non_negative
        CHECK (saturated_fat_g IS NULL OR saturated_fat_g >= 0),
    CONSTRAINT product_fibre_g_non_negative
        CHECK (fibre_g IS NULL OR fibre_g >= 0),
    CONSTRAINT product_salt_g_non_negative
        CHECK (salt_g IS NULL OR salt_g >= 0),
    CONSTRAINT product_cholesterol_mg_non_negative
        CHECK (cholesterol_mg IS NULL OR cholesterol_mg >= 0),
    CONSTRAINT product_seeded_has_key
        CHECK (origin <> 'seeded' OR seed_key IS NOT NULL)
);

CREATE UNIQUE INDEX product_barcode_unique ON product (barcode) WHERE barcode IS NOT NULL;
CREATE UNIQUE INDEX product_seed_key_unique ON product (seed_key) WHERE seed_key IS NOT NULL;
CREATE INDEX product_name_trgm ON product USING GIN (name gin_trgm_ops);
CREATE INDEX product_mapped_ingredient ON product (mapped_ingredient_id)
    WHERE mapped_ingredient_id IS NOT NULL;

CREATE TABLE app_user (
    id            UUID PRIMARY KEY,
    username      TEXT NOT NULL,
    display_name  TEXT,
    auth_subject  TEXT,

    revision      BIGINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at   TIMESTAMPTZ,

    CONSTRAINT app_user_username_not_blank
        CHECK (btrim(username) <> ''),
    CONSTRAINT app_user_display_name_not_blank
        CHECK (display_name IS NULL OR btrim(display_name) <> '')
);

CREATE UNIQUE INDEX app_user_username_unique ON app_user (lower(username));
CREATE UNIQUE INDEX app_user_auth_subject_unique ON app_user (auth_subject)
    WHERE auth_subject IS NOT NULL;

CREATE TABLE household_member (
    id              UUID PRIMARY KEY,
    display_name    TEXT NOT NULL,
    linked_user_id  UUID UNIQUE REFERENCES app_user (id) ON DELETE SET NULL,

    revision        BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at     TIMESTAMPTZ,

    CONSTRAINT household_member_display_name_not_blank
        CHECK (btrim(display_name) <> '')
);

CREATE UNIQUE INDEX household_member_display_name_unique
    ON household_member (lower(display_name));

CREATE TABLE user_role (
    user_id  UUID NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    role     TEXT NOT NULL,

    PRIMARY KEY (user_id, role),
    CONSTRAINT user_role_valid
        CHECK (role IN ('admin', 'household_manager', 'nutritionist', 'basic_user'))
);

CREATE TABLE member_access_grant (
    grantee_user_id    UUID NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    subject_member_id  UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    scope              TEXT NOT NULL,
    granted_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    granted_by         UUID REFERENCES app_user (id) ON DELETE SET NULL,

    PRIMARY KEY (grantee_user_id, subject_member_id, scope),
    CONSTRAINT member_access_grant_scope_valid
        CHECK (scope IN ('health_data'))
);

CREATE INDEX member_access_grant_subject ON member_access_grant (subject_member_id);

ALTER TABLE ingredient
    ADD COLUMN created_by UUID REFERENCES app_user (id) ON DELETE SET NULL,
    ADD COLUMN updated_by UUID REFERENCES app_user (id) ON DELETE SET NULL;

ALTER TABLE product
    ADD COLUMN created_by UUID REFERENCES app_user (id) ON DELETE SET NULL,
    ADD COLUMN updated_by UUID REFERENCES app_user (id) ON DELETE SET NULL;

CREATE TABLE consumption_record (
    id            UUID PRIMARY KEY,
    member_id     UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    product_id    UUID NOT NULL REFERENCES product (id) ON DELETE RESTRICT,
    recorded_by   UUID REFERENCES app_user (id) ON DELETE SET NULL,

    amount_kind   TEXT NOT NULL,
    amount_value  NUMERIC(16, 4) NOT NULL,
    amount_unit   TEXT,

    consumed_on   DATE NOT NULL,
    consumed_at   TIMESTAMPTZ,
    slot          TEXT NOT NULL,

    nutrition_basis_amount  NUMERIC(16, 4),
    nutrition_basis_unit    TEXT,
    energy_kcal         NUMERIC(12, 3),
    protein_g           NUMERIC(12, 3),
    carbohydrate_g      NUMERIC(12, 3),
    sugar_g             NUMERIC(12, 3),
    fat_g               NUMERIC(12, 3),
    saturated_fat_g     NUMERIC(12, 3),
    fibre_g             NUMERIC(12, 3),
    salt_g              NUMERIC(12, 3),
    cholesterol_mg      NUMERIC(12, 3),
    nutrition_extra     JSONB NOT NULL DEFAULT '{}'::jsonb,
    nutrition_quality   TEXT NOT NULL,

    revision      BIGINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT consumption_record_amount_kind_valid
        CHECK (amount_kind IN ('measure', 'servings', 'packs')),
    CONSTRAINT consumption_record_amount_value_positive
        CHECK (amount_value > 0),
    CONSTRAINT consumption_record_amount_unit_present
        CHECK ((amount_kind = 'measure') = (amount_unit IS NOT NULL)),
    CONSTRAINT consumption_record_amount_unit_valid
        CHECK (amount_unit IS NULL OR amount_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch')),
    CONSTRAINT consumption_record_basis_unit_valid
        CHECK (nutrition_basis_unit IS NULL OR nutrition_basis_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch')),
    CONSTRAINT consumption_record_basis_complete
        CHECK (num_nonnulls(nutrition_basis_amount, nutrition_basis_unit) <> 1),
    CONSTRAINT consumption_record_basis_positive
        CHECK (nutrition_basis_amount IS NULL OR nutrition_basis_amount > 0),
    CONSTRAINT consumption_record_extra_is_object
        CHECK (jsonb_typeof(nutrition_extra) = 'object'),
    CONSTRAINT consumption_record_energy_kcal_non_negative
        CHECK (energy_kcal IS NULL OR energy_kcal >= 0),
    CONSTRAINT consumption_record_protein_g_non_negative
        CHECK (protein_g IS NULL OR protein_g >= 0),
    CONSTRAINT consumption_record_carbohydrate_g_non_negative
        CHECK (carbohydrate_g IS NULL OR carbohydrate_g >= 0),
    CONSTRAINT consumption_record_sugar_g_non_negative
        CHECK (sugar_g IS NULL OR sugar_g >= 0),
    CONSTRAINT consumption_record_fat_g_non_negative
        CHECK (fat_g IS NULL OR fat_g >= 0),
    CONSTRAINT consumption_record_saturated_fat_g_non_negative
        CHECK (saturated_fat_g IS NULL OR saturated_fat_g >= 0),
    CONSTRAINT consumption_record_fibre_g_non_negative
        CHECK (fibre_g IS NULL OR fibre_g >= 0),
    CONSTRAINT consumption_record_salt_g_non_negative
        CHECK (salt_g IS NULL OR salt_g >= 0),
    CONSTRAINT consumption_record_cholesterol_mg_non_negative
        CHECK (cholesterol_mg IS NULL OR cholesterol_mg >= 0),
    CONSTRAINT consumption_record_quality_valid
        CHECK (nutrition_quality IN ('known', 'estimated', 'partial', 'unknown')),
    CONSTRAINT consumption_record_slot_valid
        CHECK (slot IN ('breakfast', 'lunch', 'dinner', 'snacks'))
);

CREATE INDEX consumption_record_member_day ON consumption_record (member_id, consumed_on);
CREATE INDEX consumption_record_product ON consumption_record (product_id);

CREATE TABLE meal_plan_entry (
    id            UUID PRIMARY KEY,
    member_id     UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    planned_on    DATE NOT NULL,
    planned_time  TIME,
    slot          TEXT NOT NULL,
    status        TEXT NOT NULL,
    created_by    UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    updated_by    UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    resolved_by   UUID REFERENCES app_user (id) ON DELETE RESTRICT,
    resolved_at   TIMESTAMPTZ,
    revision      BIGINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT meal_plan_entry_slot_valid
        CHECK (slot IN ('breakfast', 'lunch', 'dinner', 'snacks')),
    CONSTRAINT meal_plan_entry_status_valid
        CHECK (status IN ('planned', 'partially_resolved', 'eaten', 'not_eaten')),
    CONSTRAINT meal_plan_entry_resolution_complete
        CHECK ((status = 'planned') = (resolved_by IS NULL AND resolved_at IS NULL))
);

CREATE INDEX meal_plan_entry_member_day
    ON meal_plan_entry (member_id, planned_on, slot, planned_time);
CREATE UNIQUE INDEX meal_plan_entry_member_day_slot_unique
    ON meal_plan_entry (member_id, planned_on, slot);
CREATE INDEX meal_plan_entry_status_day
    ON meal_plan_entry (status, planned_on);

CREATE TABLE meal_plan_component (
    id            UUID PRIMARY KEY,
    entry_id      UUID NOT NULL REFERENCES meal_plan_entry (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    product_id    UUID NOT NULL REFERENCES product (id) ON DELETE RESTRICT,
    amount_kind   TEXT NOT NULL,
    amount_value  NUMERIC(16, 4) NOT NULL,
    amount_unit   TEXT,
    frozen_product_name TEXT,
    nutrition_basis_amount  NUMERIC(16, 4),
    nutrition_basis_unit    TEXT,
    energy_kcal         NUMERIC(12, 3),
    protein_g           NUMERIC(12, 3),
    carbohydrate_g      NUMERIC(12, 3),
    sugar_g             NUMERIC(12, 3),
    fat_g               NUMERIC(12, 3),
    saturated_fat_g     NUMERIC(12, 3),
    fibre_g             NUMERIC(12, 3),
    salt_g              NUMERIC(12, 3),
    cholesterol_mg      NUMERIC(12, 3),
    nutrition_extra     JSONB,
    nutrition_quality   TEXT,
    status          TEXT NOT NULL DEFAULT 'planned',
    resolved_by     UUID REFERENCES app_user (id) ON DELETE RESTRICT,
    resolved_at     TIMESTAMPTZ,
    revision        BIGINT NOT NULL DEFAULT 1,
    display_order   UUID NOT NULL,

    CONSTRAINT meal_plan_component_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT meal_plan_component_status_valid
        CHECK (status IN ('planned', 'eaten', 'not_eaten')),
    CONSTRAINT meal_plan_component_resolution_complete
        CHECK ((status = 'planned') = (resolved_by IS NULL AND resolved_at IS NULL)),
    CONSTRAINT meal_plan_component_amount_kind_valid
        CHECK (amount_kind IN ('measure', 'servings', 'packs')),
    CONSTRAINT meal_plan_component_amount_value_positive
        CHECK (amount_value > 0),
    CONSTRAINT meal_plan_component_amount_unit_present
        CHECK ((amount_kind = 'measure') = (amount_unit IS NOT NULL)),
    CONSTRAINT meal_plan_component_amount_unit_valid
        CHECK (amount_unit IS NULL OR amount_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch')),
    CONSTRAINT meal_plan_component_basis_unit_valid
        CHECK (nutrition_basis_unit IS NULL OR nutrition_basis_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch')),
    CONSTRAINT meal_plan_component_basis_complete
        CHECK (num_nonnulls(nutrition_basis_amount, nutrition_basis_unit) <> 1),
    CONSTRAINT meal_plan_component_basis_positive
        CHECK (nutrition_basis_amount IS NULL OR nutrition_basis_amount > 0),
    CONSTRAINT meal_plan_component_snapshot_complete
        CHECK (
            (frozen_product_name IS NULL AND nutrition_quality IS NULL AND nutrition_extra IS NULL)
            OR
            (frozen_product_name IS NOT NULL AND nutrition_quality IS NOT NULL AND nutrition_extra IS NOT NULL)
        ),
    CONSTRAINT meal_plan_component_extra_is_object
        CHECK (nutrition_extra IS NULL OR jsonb_typeof(nutrition_extra) = 'object'),
    CONSTRAINT meal_plan_component_quality_valid
        CHECK (nutrition_quality IS NULL OR nutrition_quality IN ('known', 'estimated', 'partial', 'unknown')),
    CONSTRAINT meal_plan_component_energy_non_negative
        CHECK (energy_kcal IS NULL OR energy_kcal >= 0),
    CONSTRAINT meal_plan_component_protein_non_negative
        CHECK (protein_g IS NULL OR protein_g >= 0),
    CONSTRAINT meal_plan_component_carbohydrate_non_negative
        CHECK (carbohydrate_g IS NULL OR carbohydrate_g >= 0),
    CONSTRAINT meal_plan_component_sugar_non_negative
        CHECK (sugar_g IS NULL OR sugar_g >= 0),
    CONSTRAINT meal_plan_component_fat_non_negative
        CHECK (fat_g IS NULL OR fat_g >= 0),
    CONSTRAINT meal_plan_component_saturated_fat_non_negative
        CHECK (saturated_fat_g IS NULL OR saturated_fat_g >= 0),
    CONSTRAINT meal_plan_component_fibre_non_negative
        CHECK (fibre_g IS NULL OR fibre_g >= 0),
    CONSTRAINT meal_plan_component_salt_non_negative
        CHECK (salt_g IS NULL OR salt_g >= 0),
    CONSTRAINT meal_plan_component_cholesterol_non_negative
        CHECK (cholesterol_mg IS NULL OR cholesterol_mg >= 0),
    UNIQUE (entry_id, position)
);

CREATE INDEX meal_plan_component_entry ON meal_plan_component (entry_id, position);
CREATE INDEX meal_plan_component_product ON meal_plan_component (product_id);

ALTER TABLE consumption_record
    ADD COLUMN meal_plan_component_id UUID
        REFERENCES meal_plan_component (id) ON DELETE RESTRICT;

CREATE UNIQUE INDEX consumption_record_meal_plan_component_unique
    ON consumption_record (meal_plan_component_id)
    WHERE meal_plan_component_id IS NOT NULL;

CREATE TABLE nutrition_target (
    id              UUID PRIMARY KEY,
    member_id       UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    effective_from  DATE NOT NULL,

    energy_kcal         NUMERIC(12, 3),
    protein_g           NUMERIC(12, 3),
    carbohydrate_g      NUMERIC(12, 3),
    sugar_g             NUMERIC(12, 3),
    fat_g               NUMERIC(12, 3),
    saturated_fat_g     NUMERIC(12, 3),
    fibre_g             NUMERIC(12, 3),
    salt_g              NUMERIC(12, 3),
    cholesterol_mg      NUMERIC(12, 3),

    revision        BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT nutrition_target_has_goal
        CHECK (num_nonnulls(energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g, saturated_fat_g, fibre_g, salt_g, cholesterol_mg) > 0),
    CONSTRAINT nutrition_target_energy_kcal_non_negative
        CHECK (energy_kcal IS NULL OR energy_kcal >= 0),
    CONSTRAINT nutrition_target_protein_g_non_negative
        CHECK (protein_g IS NULL OR protein_g >= 0),
    CONSTRAINT nutrition_target_carbohydrate_g_non_negative
        CHECK (carbohydrate_g IS NULL OR carbohydrate_g >= 0),
    CONSTRAINT nutrition_target_sugar_g_non_negative
        CHECK (sugar_g IS NULL OR sugar_g >= 0),
    CONSTRAINT nutrition_target_fat_g_non_negative
        CHECK (fat_g IS NULL OR fat_g >= 0),
    CONSTRAINT nutrition_target_saturated_fat_g_non_negative
        CHECK (saturated_fat_g IS NULL OR saturated_fat_g >= 0),
    CONSTRAINT nutrition_target_fibre_g_non_negative
        CHECK (fibre_g IS NULL OR fibre_g >= 0),
    CONSTRAINT nutrition_target_salt_g_non_negative
        CHECK (salt_g IS NULL OR salt_g >= 0),
    CONSTRAINT nutrition_target_cholesterol_mg_non_negative
        CHECK (cholesterol_mg IS NULL OR cholesterol_mg >= 0)
);

CREATE UNIQUE INDEX nutrition_target_member_effective_from_unique
    ON nutrition_target (member_id, effective_from);

CREATE TABLE household_settings (
    singleton       BOOLEAN PRIMARY KEY DEFAULT TRUE,

    breakfast_time  TIME NOT NULL,
    lunch_time      TIME NOT NULL,
    dinner_time     TIME NOT NULL,

    revision        BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT household_settings_singleton
        CHECK (singleton)
);

INSERT INTO household_settings (singleton, breakfast_time, lunch_time, dinner_time)
VALUES (TRUE, '08:00', '12:30', '18:00');

UPDATE meal_plan_entry
SET planned_time = NULL
WHERE slot = 'snacks' AND planned_time IS NOT NULL;

ALTER TABLE meal_plan_entry
    ADD CONSTRAINT meal_plan_entry_snacks_have_no_planned_time
        CHECK (slot <> 'snacks' OR planned_time IS NULL);

CREATE TABLE recipe (
    id            UUID PRIMARY KEY,
    name          TEXT NOT NULL,
    description   TEXT,
    servings      INTEGER NOT NULL,
    preparation_minutes INTEGER,
    cooking_minutes INTEGER,
    notes         TEXT,
    photo_version BIGINT,
    owner_id      UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    visibility    TEXT NOT NULL DEFAULT 'private',

    created_by    UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    updated_by    UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    revision      BIGINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at   TIMESTAMPTZ,

    CONSTRAINT recipe_name_not_blank
        CHECK (btrim(name) <> ''),
    CONSTRAINT recipe_servings_positive
        CHECK (servings > 0),
    CONSTRAINT recipe_preparation_minutes_positive
        CHECK (preparation_minutes IS NULL OR preparation_minutes > 0),
    CONSTRAINT recipe_cooking_minutes_positive
        CHECK (cooking_minutes IS NULL OR cooking_minutes > 0),
    CONSTRAINT recipe_photo_version_positive
        CHECK (photo_version IS NULL OR photo_version > 0),
    CONSTRAINT recipe_visibility_valid
        CHECK (visibility IN ('private', 'shared'))
);

CREATE INDEX recipe_owner ON recipe (owner_id, archived_at);
CREATE INDEX recipe_name_trgm ON recipe USING gin (name gin_trgm_ops);

CREATE TABLE recipe_component (
    id               UUID PRIMARY KEY,
    recipe_id        UUID NOT NULL REFERENCES recipe (id) ON DELETE CASCADE,
    position         INTEGER NOT NULL,
    ingredient_id    UUID REFERENCES ingredient (id) ON DELETE RESTRICT,
    product_id       UUID REFERENCES product (id) ON DELETE RESTRICT,
    unresolved_text  TEXT,
    source_text      TEXT,
    amount_kind      TEXT NOT NULL,
    amount_value     NUMERIC(16, 4) NOT NULL,
    amount_unit      TEXT,

    CONSTRAINT recipe_component_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT recipe_component_requirement_exclusive
        CHECK (num_nonnulls(ingredient_id, product_id, unresolved_text) = 1),
    CONSTRAINT recipe_component_unresolved_text_not_blank
        CHECK (unresolved_text IS NULL OR btrim(unresolved_text) <> ''),
    CONSTRAINT recipe_component_source_text_not_blank
        CHECK (source_text IS NULL OR btrim(source_text) <> ''),
    CONSTRAINT recipe_component_source_text_not_unresolved
        CHECK (source_text IS NULL OR unresolved_text IS NULL),
    CONSTRAINT recipe_component_amount_kind_valid
        CHECK (amount_kind IN ('measure', 'servings', 'packs')),
    CONSTRAINT recipe_component_relative_amount_needs_product
        CHECK (amount_kind = 'measure' OR product_id IS NOT NULL),
    CONSTRAINT recipe_component_amount_value_positive
        CHECK (amount_value > 0),
    CONSTRAINT recipe_component_amount_unit_present
        CHECK ((amount_kind = 'measure') = (amount_unit IS NOT NULL)),
    CONSTRAINT recipe_component_amount_unit_valid
        CHECK (amount_unit IS NULL OR amount_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch')),
    UNIQUE (recipe_id, position)
);

CREATE INDEX recipe_component_recipe ON recipe_component (recipe_id, position);
CREATE INDEX recipe_component_product ON recipe_component (product_id) WHERE product_id IS NOT NULL;
CREATE INDEX recipe_component_ingredient ON recipe_component (ingredient_id) WHERE ingredient_id IS NOT NULL;

CREATE TABLE recipe_instruction (
    id            UUID PRIMARY KEY,
    recipe_id     UUID NOT NULL REFERENCES recipe (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    instruction   TEXT NOT NULL,

    CONSTRAINT recipe_instruction_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT recipe_instruction_not_blank
        CHECK (btrim(instruction) <> ''),
    UNIQUE (recipe_id, position)
);

CREATE INDEX recipe_instruction_recipe ON recipe_instruction (recipe_id, position);

CREATE TABLE recipe_meal_category (
    recipe_id     UUID NOT NULL REFERENCES recipe (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    category      TEXT NOT NULL,

    CONSTRAINT recipe_meal_category_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT recipe_meal_category_valid
        CHECK (category IN ('breakfast', 'lunch', 'dinner', 'snack')),
    PRIMARY KEY (recipe_id, category),
    UNIQUE (recipe_id, position)
);

CREATE TABLE recipe_country_category (
    recipe_id     UUID NOT NULL REFERENCES recipe (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    country_code  TEXT NOT NULL,

    CONSTRAINT recipe_country_category_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT recipe_country_category_code_valid
        CHECK (country_code ~ '^[A-Z]{2}$'),
    PRIMARY KEY (recipe_id, country_code),
    UNIQUE (recipe_id, position)
);

CREATE TABLE recipe_tag (
    recipe_id     UUID NOT NULL REFERENCES recipe (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    tag           TEXT NOT NULL,

    CONSTRAINT recipe_tag_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT recipe_tag_not_blank
        CHECK (btrim(tag) <> ''),
    PRIMARY KEY (recipe_id, tag),
    UNIQUE (recipe_id, position)
);

CREATE UNIQUE INDEX recipe_tag_case_insensitive
    ON recipe_tag (recipe_id, lower(tag));

CREATE TABLE recipe_photo (
    recipe_id     UUID PRIMARY KEY REFERENCES recipe (id) ON DELETE CASCADE,
    version       BIGINT NOT NULL,
    hero_jpeg     BYTEA NOT NULL,
    card_jpeg     BYTEA NOT NULL,
    hero_width    INTEGER NOT NULL,
    hero_height   INTEGER NOT NULL,
    card_width    INTEGER NOT NULL,
    card_height   INTEGER NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL,

    CONSTRAINT recipe_photo_version_positive
        CHECK (version > 0),
    CONSTRAINT recipe_photo_dimensions_positive
        CHECK (hero_width > 0 AND hero_height > 0 AND card_width > 0 AND card_height > 0),
    CONSTRAINT recipe_photo_bytes_present
        CHECK (octet_length(hero_jpeg) > 0 AND octet_length(card_jpeg) > 0)
);

ALTER TABLE meal_plan_component
    RENAME COLUMN frozen_product_name TO frozen_item_name;

ALTER TABLE meal_plan_component
    ADD COLUMN item_kind TEXT NOT NULL DEFAULT 'product',
    ADD COLUMN recipe_id UUID REFERENCES recipe (id) ON DELETE RESTRICT,
    ALTER COLUMN product_id DROP NOT NULL,
    ADD CONSTRAINT meal_plan_component_item_kind_valid
        CHECK (item_kind IN ('product', 'recipe')),
    ADD CONSTRAINT meal_plan_component_item_ref_exclusive
        CHECK (
            num_nonnulls(product_id, recipe_id) = 1
            AND (item_kind = 'product') = (product_id IS NOT NULL)
        );

CREATE INDEX meal_plan_component_recipe ON meal_plan_component (recipe_id);

ALTER TABLE consumption_record
    ADD COLUMN item_kind TEXT NOT NULL DEFAULT 'product',
    ADD COLUMN recipe_id UUID REFERENCES recipe (id) ON DELETE RESTRICT,
    ALTER COLUMN product_id DROP NOT NULL,
    ADD CONSTRAINT consumption_record_item_kind_valid
        CHECK (item_kind IN ('product', 'recipe')),
    ADD CONSTRAINT consumption_record_item_ref_exclusive
        CHECK (
            num_nonnulls(product_id, recipe_id) = 1
            AND (item_kind = 'product') = (product_id IS NOT NULL)
        );

CREATE INDEX consumption_record_recipe ON consumption_record (recipe_id);

ALTER TABLE household_settings
    ADD COLUMN missing_stock_interpretation TEXT NOT NULL DEFAULT 'unknown',
    ADD CONSTRAINT household_settings_missing_stock_interpretation_valid
        CHECK (missing_stock_interpretation IN ('absent', 'unknown'));

CREATE TABLE stock_item (
    id                UUID PRIMARY KEY,
    product_id        UUID NOT NULL REFERENCES product (id) ON DELETE RESTRICT,

    tracking_mode     TEXT NOT NULL,
    quantity_value    NUMERIC(16, 4),
    quantity_unit     TEXT,
    estimated_low     NUMERIC(16, 4),
    estimated_high    NUMERIC(16, 4),

    storage_location  TEXT NOT NULL,

    source_date       DATE,
    source_date_kind  TEXT,
    usability_deadline DATE,
    usability_deadline_basis TEXT,

    note              TEXT,

    revision          BIGINT NOT NULL DEFAULT 1,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at       TIMESTAMPTZ,

    CONSTRAINT stock_item_tracking_mode_valid
        CHECK (tracking_mode IN ('exact', 'estimated', 'not_tracked')),
    CONSTRAINT stock_item_storage_location_valid
        CHECK (storage_location IN ('ambient', 'chilled', 'frozen')),
    CONSTRAINT stock_item_quantity_unit_valid
        CHECK (quantity_unit IS NULL OR quantity_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch')),
    CONSTRAINT stock_item_exact_has_quantity
        CHECK (tracking_mode <> 'exact'
            OR (quantity_value IS NOT NULL AND quantity_unit IS NOT NULL
                AND estimated_low IS NULL AND estimated_high IS NULL)),
    CONSTRAINT stock_item_estimated_has_band
        CHECK (tracking_mode <> 'estimated'
            OR (quantity_unit IS NOT NULL AND estimated_low IS NOT NULL AND estimated_high IS NOT NULL
                AND quantity_value IS NULL)),
    CONSTRAINT stock_item_not_tracked_has_nothing
        CHECK (tracking_mode <> 'not_tracked'
            OR (quantity_value IS NULL AND estimated_low IS NULL AND estimated_high IS NULL)),
    CONSTRAINT stock_item_estimated_band_ordered
        CHECK (estimated_low IS NULL OR estimated_high IS NULL OR estimated_low <= estimated_high),
    CONSTRAINT stock_item_quantities_non_negative
        CHECK ((quantity_value IS NULL OR quantity_value >= 0)
            AND (estimated_low IS NULL OR estimated_low >= 0)
            AND (estimated_high IS NULL OR estimated_high >= 0)),
    CONSTRAINT stock_item_source_date_kind_valid
        CHECK (source_date_kind IS NULL OR source_date_kind IN ('use_by', 'best_before'))
);

CREATE INDEX stock_item_product ON stock_item (product_id);
CREATE INDEX stock_item_usability_deadline ON stock_item (usability_deadline)
    WHERE usability_deadline IS NOT NULL AND archived_at IS NULL;

CREATE TABLE stock_event (
    id                UUID PRIMARY KEY,
    stock_item_id     UUID NOT NULL REFERENCES stock_item (id) ON DELETE RESTRICT,

    event_kind        TEXT NOT NULL,
    quantity_delta    NUMERIC(16, 4),
    quantity_unit     TEXT,

    actor_user_id     UUID REFERENCES app_user (id) ON DELETE SET NULL,
    subject_member_id UUID REFERENCES household_member (id) ON DELETE SET NULL,

    note              TEXT,
    occurred_at       TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT stock_event_kind_valid
        CHECK (event_kind IN ('added', 'consumed', 'discarded', 'corrected', 'observed',
                              'moved', 'mode_changed', 'archived')),
    CONSTRAINT stock_event_unit_valid
        CHECK (quantity_unit IS NULL OR quantity_unit IN ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch'))
);

CREATE INDEX stock_event_item ON stock_event (stock_item_id, occurred_at DESC);
