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

CREATE TABLE idempotency_key (
    key                  TEXT PRIMARY KEY,
    principal            TEXT NOT NULL,
    request_fingerprint  TEXT NOT NULL,
    response_status      INTEGER NOT NULL,
    response_body        JSONB,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

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
    consumed_at   TIMESTAMPTZ NOT NULL,

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
        CHECK (nutrition_quality IN ('known', 'partial', 'unknown'))
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
        CHECK (status IN ('planned', 'eaten', 'not_eaten')),
    CONSTRAINT meal_plan_entry_resolution_complete
        CHECK ((status = 'planned') = (resolved_by IS NULL AND resolved_at IS NULL))
);

CREATE INDEX meal_plan_entry_member_day
    ON meal_plan_entry (member_id, planned_on, slot, planned_time);
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

    CONSTRAINT meal_plan_component_position_non_negative
        CHECK (position >= 0),
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
        CHECK (nutrition_quality IS NULL OR nutrition_quality IN ('known', 'partial', 'unknown')),
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
