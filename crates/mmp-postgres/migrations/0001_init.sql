CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE DOMAIN unit_code AS TEXT
    CONSTRAINT unit_code_valid CHECK (VALUE IN
        ('mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item',
         'piece', 'slice', 'clove', 'can', 'pack', 'bunch', 'serving'));

CREATE DOMAIN shopping_section_code AS TEXT
    CONSTRAINT shopping_section_code_valid CHECK (VALUE IN
        ('fresh_produce', 'meat_fish', 'dairy', 'bakery', 'frozen', 'ambient', 'drinks',
         'household', 'other'));

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

CREATE TABLE household_member (
    id              UUID PRIMARY KEY,
    display_name    TEXT NOT NULL,
    linked_user_id  UUID UNIQUE REFERENCES app_user (id) ON DELETE SET NULL,
    weight_display  TEXT NOT NULL DEFAULT 'kilograms',

    revision        BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at     TIMESTAMPTZ,

    CONSTRAINT household_member_display_name_not_blank
        CHECK (btrim(display_name) <> ''),
    CONSTRAINT household_member_weight_display_known
        CHECK (weight_display IN ('kilograms', 'stones_pounds', 'pounds'))
);

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
        CHECK (scope IN ('health_data', 'meal_plan'))
);

CREATE TABLE ingredient (
    id                  UUID PRIMARY KEY,
    name                TEXT NOT NULL,
    default_unit        unit_code NOT NULL,
    shopping_section    shopping_section_code,
    track_stock         BOOLEAN,

    origin              TEXT NOT NULL,
    seed_key            TEXT,
    source_provider     TEXT,
    source_external_id  TEXT,
    locally_modified    BOOLEAN NOT NULL DEFAULT FALSE,

    revision            BIGINT NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at         TIMESTAMPTZ,
    created_by          UUID REFERENCES app_user (id) ON DELETE SET NULL,
    updated_by          UUID REFERENCES app_user (id) ON DELETE SET NULL,

    CONSTRAINT ingredient_name_not_blank
        CHECK (btrim(name) <> ''),
    CONSTRAINT ingredient_origin_valid
        CHECK (origin IN ('seeded', 'local', 'external')),
    CONSTRAINT ingredient_seeded_has_key
        CHECK (origin <> 'seeded' OR seed_key IS NOT NULL)
);

CREATE TABLE prepared_meal (
    id                  UUID PRIMARY KEY,
    name                TEXT NOT NULL,
    default_unit        unit_code NOT NULL,
    shopping_section    shopping_section_code,
    track_stock         BOOLEAN,

    origin              TEXT NOT NULL,
    seed_key            TEXT,
    source_provider     TEXT,
    source_external_id  TEXT,
    locally_modified    BOOLEAN NOT NULL DEFAULT FALSE,

    revision            BIGINT NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at         TIMESTAMPTZ,

    CONSTRAINT prepared_meal_name_not_blank
        CHECK (btrim(name) <> ''),
    CONSTRAINT prepared_meal_origin_valid
        CHECK (origin IN ('seeded', 'local', 'external')),
    CONSTRAINT prepared_meal_seeded_has_key
        CHECK (origin <> 'seeded' OR seed_key IS NOT NULL)
);

CREATE TABLE product (
    id                       UUID PRIMARY KEY,
    name                     TEXT NOT NULL,
    brand                    TEXT,
    barcode                  TEXT,
    retailer                 TEXT,
    shopping_section         shopping_section_code,
    package_quantity_amount  NUMERIC(16, 4),
    package_quantity_unit    unit_code,
    servings_per_pack        INTEGER,
    mapped_ingredient_id     UUID REFERENCES ingredient (id) ON DELETE RESTRICT,
    mapped_prepared_meal_id  UUID REFERENCES prepared_meal (id) ON DELETE RESTRICT,
    track_stock              BOOLEAN,

    nutrition_basis_amount  NUMERIC(16, 4),
    nutrition_basis_unit    unit_code,
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
    created_by          UUID REFERENCES app_user (id) ON DELETE SET NULL,
    updated_by          UUID REFERENCES app_user (id) ON DELETE SET NULL,

    CONSTRAINT product_name_not_blank
        CHECK (btrim(name) <> ''),
    CONSTRAINT product_origin_valid
        CHECK (origin IN ('seeded', 'local', 'external')),
    CONSTRAINT product_barcode_valid
        CHECK (barcode IS NULL OR barcode ~ '^[0-9]{4,18}$'),
    CONSTRAINT product_package_quantity_complete
        CHECK (num_nonnulls(package_quantity_amount, package_quantity_unit) <> 1),
    CONSTRAINT product_package_quantity_positive
        CHECK (package_quantity_amount IS NULL OR package_quantity_amount > 0),
    CONSTRAINT product_servings_per_pack_positive
        CHECK (servings_per_pack IS NULL OR servings_per_pack > 0),
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
        CHECK (origin <> 'seeded' OR seed_key IS NOT NULL),
    CONSTRAINT product_mapping_exclusive
        CHECK (num_nonnulls(mapped_ingredient_id, mapped_prepared_meal_id) <= 1)
);

CREATE TABLE household_settings (
    singleton       BOOLEAN PRIMARY KEY DEFAULT TRUE,

    breakfast_time  TIME NOT NULL,
    lunch_time      TIME NOT NULL,
    dinner_time     TIME NOT NULL,
    timezone        TEXT NOT NULL DEFAULT 'Etc/UTC',
    missing_stock_interpretation TEXT NOT NULL DEFAULT 'unknown',
    assume_eaten_when_time_passes BOOLEAN NOT NULL DEFAULT TRUE,
    shopping_section_order shopping_section_code[] NOT NULL
        DEFAULT ARRAY['fresh_produce', 'meat_fish', 'dairy', 'bakery', 'frozen', 'ambient',
                      'drinks', 'household', 'other']::shopping_section_code[],

    revision        BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT household_settings_singleton
        CHECK (singleton),
    CONSTRAINT household_settings_missing_stock_interpretation_valid
        CHECK (missing_stock_interpretation IN ('absent', 'unknown')),
    CONSTRAINT household_settings_section_order_complete
        CHECK (cardinality(shopping_section_order) = 9)
);

CREATE TABLE nutrition_target (
    id              UUID PRIMARY KEY,
    member_id       UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    effective_from  DATE NOT NULL,
    source          TEXT NOT NULL DEFAULT 'user_defined',

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
        CHECK (cholesterol_mg IS NULL OR cholesterol_mg >= 0),
    CONSTRAINT nutrition_target_source_known
        CHECK (source IN ('user_defined', 'calculated'))
);

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
    amount_unit      unit_code,

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
    CONSTRAINT recipe_component_recipe_id_position_unique UNIQUE (recipe_id, position)
);

CREATE TABLE recipe_instruction (
    id            UUID PRIMARY KEY,
    recipe_id     UUID NOT NULL REFERENCES recipe (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    instruction   TEXT NOT NULL,

    CONSTRAINT recipe_instruction_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT recipe_instruction_not_blank
        CHECK (btrim(instruction) <> ''),
    CONSTRAINT recipe_instruction_recipe_id_position_unique UNIQUE (recipe_id, position)
);

CREATE TABLE recipe_meal_category (
    recipe_id     UUID NOT NULL REFERENCES recipe (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    category      TEXT NOT NULL,

    CONSTRAINT recipe_meal_category_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT recipe_meal_category_valid
        CHECK (category IN ('breakfast', 'lunch', 'dinner', 'snack')),
    PRIMARY KEY (recipe_id, category),
    CONSTRAINT recipe_meal_category_recipe_id_position_unique UNIQUE (recipe_id, position)
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
    CONSTRAINT recipe_country_category_recipe_id_position_unique UNIQUE (recipe_id, position)
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
    CONSTRAINT recipe_tag_recipe_id_position_unique UNIQUE (recipe_id, position)
);

CREATE TABLE recipe_photo (
    recipe_id     UUID PRIMARY KEY REFERENCES recipe (id) ON DELETE CASCADE,
    version       BIGINT NOT NULL,
    hero_jpeg     BYTEA NOT NULL,
    card_jpeg     BYTEA NOT NULL,
    hero_width    INTEGER NOT NULL,
    hero_height   INTEGER NOT NULL,
    card_width    INTEGER NOT NULL,
    card_height   INTEGER NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT recipe_photo_version_positive
        CHECK (version > 0),
    CONSTRAINT recipe_photo_dimensions_positive
        CHECK (hero_width > 0 AND hero_height > 0 AND card_width > 0 AND card_height > 0),
    CONSTRAINT recipe_photo_bytes_present
        CHECK (octet_length(hero_jpeg) > 0 AND octet_length(card_jpeg) > 0)
);

CREATE TABLE meal_occasion (
    id            UUID PRIMARY KEY,
    planned_on    DATE NOT NULL,
    slot          TEXT NOT NULL,
    planned_time  TIME,
    note          TEXT,
    created_by    UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    updated_by    UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    revision      BIGINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT meal_occasion_slot_valid
        CHECK (slot IN ('breakfast', 'lunch', 'dinner', 'snacks')),
    CONSTRAINT meal_occasion_note_not_blank
        CHECK (note IS NULL OR btrim(note) <> ''),
    CONSTRAINT meal_occasion_planned_on_slot_unique
        UNIQUE (planned_on, slot)
);

CREATE TABLE meal_occasion_absence (
    occasion_id   UUID NOT NULL REFERENCES meal_occasion (id) ON DELETE CASCADE,
    member_id     UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    created_by    UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (occasion_id, member_id)
);

CREATE TABLE meal_plan_entry (
    id                UUID PRIMARY KEY,
    occasion_id       UUID NOT NULL REFERENCES meal_occasion (id) ON DELETE CASCADE,
    label             TEXT,
    ad_hoc            TEXT,
    everyone          BOOLEAN NOT NULL DEFAULT TRUE,
    cooking_servings  INTEGER,
    created_by        UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    updated_by        UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    revision          BIGINT NOT NULL DEFAULT 1,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT meal_plan_entry_ad_hoc_valid
        CHECK (ad_hoc IS NULL OR ad_hoc IN ('eating_out', 'takeaway', 'fend_for_yourself')),
    CONSTRAINT meal_plan_entry_label_not_blank
        CHECK (label IS NULL OR btrim(label) <> ''),
    CONSTRAINT meal_plan_entry_cooking_servings_positive
        CHECK (cooking_servings IS NULL OR cooking_servings > 0)
);

CREATE TABLE meal_plan_component (
    id            UUID PRIMARY KEY,
    entry_id      UUID NOT NULL REFERENCES meal_plan_entry (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    item_kind     TEXT NOT NULL DEFAULT 'product',
    product_id    UUID REFERENCES product (id) ON DELETE RESTRICT,
    recipe_id     UUID,
    ingredient_id UUID REFERENCES ingredient (id) ON DELETE RESTRICT,
    prepared_meal_id UUID REFERENCES prepared_meal (id) ON DELETE RESTRICT,
    amount_kind   TEXT NOT NULL,
    amount_value  NUMERIC(16, 4) NOT NULL,
    amount_unit   unit_code,
    frozen_item_name TEXT,
    nutrition_basis_amount  NUMERIC(16, 4),
    nutrition_basis_unit    unit_code,
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
    revision        BIGINT NOT NULL DEFAULT 1,
    display_order   UUID NOT NULL,

    CONSTRAINT meal_plan_component_recipe_id_fk
        FOREIGN KEY (recipe_id) REFERENCES recipe (id) ON DELETE RESTRICT,
    CONSTRAINT meal_plan_component_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT meal_plan_component_item_kind_valid
        CHECK (item_kind IN ('product', 'recipe', 'dish', 'ingredient', 'prepared_meal')),
    CONSTRAINT meal_plan_component_item_ref_exclusive
        CHECK (
            num_nonnulls(product_id, recipe_id, ingredient_id, prepared_meal_id) = 1
            AND (item_kind = 'product')            = (product_id IS NOT NULL)
            AND (item_kind IN ('recipe', 'dish'))  = (recipe_id IS NOT NULL)
            AND (item_kind = 'ingredient')         = (ingredient_id IS NOT NULL)
            AND (item_kind = 'prepared_meal')      = (prepared_meal_id IS NOT NULL)
        ),
    CONSTRAINT meal_plan_component_amount_kind_valid
        CHECK (amount_kind IN ('measure', 'servings', 'packs')),
    CONSTRAINT meal_plan_component_amount_value_positive
        CHECK (amount_value > 0),
    CONSTRAINT meal_plan_component_amount_unit_present
        CHECK ((amount_kind = 'measure') = (amount_unit IS NOT NULL)),
    CONSTRAINT meal_plan_component_basis_complete
        CHECK (num_nonnulls(nutrition_basis_amount, nutrition_basis_unit) <> 1),
    CONSTRAINT meal_plan_component_basis_positive
        CHECK (nutrition_basis_amount IS NULL OR nutrition_basis_amount > 0),
    CONSTRAINT meal_plan_component_snapshot_complete
        CHECK (
            (frozen_item_name IS NULL AND nutrition_quality IS NULL AND nutrition_extra IS NULL)
            OR
            (frozen_item_name IS NOT NULL AND nutrition_quality IS NOT NULL AND nutrition_extra IS NOT NULL)
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
    CONSTRAINT meal_plan_component_entry_id_position_unique UNIQUE (entry_id, position),
    CONSTRAINT meal_plan_component_entry_id_id_unique UNIQUE (entry_id, id)
);

CREATE TABLE consumption_record (
    id            UUID PRIMARY KEY,
    member_id     UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    item_kind    TEXT NOT NULL DEFAULT 'product',
    product_id    UUID REFERENCES product (id) ON DELETE RESTRICT,
    recipe_id     UUID REFERENCES recipe (id) ON DELETE RESTRICT,
    ingredient_id UUID REFERENCES ingredient (id) ON DELETE RESTRICT,
    prepared_meal_id UUID REFERENCES prepared_meal (id) ON DELETE RESTRICT,
    meal_plan_entry_id UUID REFERENCES meal_plan_entry (id) ON DELETE RESTRICT,
    meal_plan_component_id UUID REFERENCES meal_plan_component (id) ON DELETE RESTRICT,
    recorded_by   UUID REFERENCES app_user (id) ON DELETE SET NULL,

    amount_kind   TEXT NOT NULL,
    amount_value  NUMERIC(16, 4) NOT NULL,
    amount_unit   unit_code,

    consumed_on   DATE NOT NULL,
    consumed_at   TIMESTAMPTZ,
    slot          TEXT NOT NULL,

    nutrition_basis_amount  NUMERIC(16, 4),
    nutrition_basis_unit    unit_code,
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
    archived_at   TIMESTAMPTZ,

    CONSTRAINT consumption_record_item_kind_valid
        CHECK (item_kind IN ('product', 'recipe', 'dish', 'ingredient', 'prepared_meal')),
    CONSTRAINT consumption_record_item_ref_exclusive
        CHECK (
            num_nonnulls(product_id, recipe_id, ingredient_id, prepared_meal_id) = 1
            AND (item_kind = 'product')            = (product_id IS NOT NULL)
            AND (item_kind IN ('recipe', 'dish'))  = (recipe_id IS NOT NULL)
            AND (item_kind = 'ingredient')         = (ingredient_id IS NOT NULL)
            AND (item_kind = 'prepared_meal')      = (prepared_meal_id IS NOT NULL)
        ),
    CONSTRAINT consumption_record_component_needs_entry
        CHECK (meal_plan_component_id IS NULL OR meal_plan_entry_id IS NOT NULL),
    CONSTRAINT consumption_record_amount_kind_valid
        CHECK (amount_kind IN ('measure', 'servings', 'packs')),
    CONSTRAINT consumption_record_amount_value_positive
        CHECK (amount_value > 0),
    CONSTRAINT consumption_record_amount_unit_present
        CHECK ((amount_kind = 'measure') = (amount_unit IS NOT NULL)),
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

CREATE TABLE meal_plan_participant (
    id          UUID PRIMARY KEY,
    entry_id    UUID NOT NULL REFERENCES meal_plan_entry (id) ON DELETE CASCADE,
    member_id   UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    note        TEXT,

    revision    BIGINT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT meal_plan_participant_note_not_blank
        CHECK (note IS NULL OR btrim(note) <> ''),
    CONSTRAINT meal_plan_participant_entry_member_unique
        UNIQUE (entry_id, member_id),
    CONSTRAINT meal_plan_participant_entry_id_unique
        UNIQUE (entry_id, id)
);

CREATE TABLE meal_plan_participant_allocation (
    id                    UUID PRIMARY KEY,
    entry_id              UUID NOT NULL,
    participant_id        UUID NOT NULL,
    component_id          UUID NOT NULL,

    allocated_kind        TEXT NOT NULL,
    allocated_value       NUMERIC(16, 4) NOT NULL,
    allocated_unit        unit_code,

    status                TEXT NOT NULL DEFAULT 'planned',
    consumption_record_id UUID REFERENCES consumption_record (id) ON DELETE SET NULL,
    resolved_by           UUID REFERENCES app_user (id) ON DELETE RESTRICT,
    resolved_at           TIMESTAMPTZ,

    CONSTRAINT meal_plan_participant_allocation_participant_fk
        FOREIGN KEY (entry_id, participant_id)
        REFERENCES meal_plan_participant (entry_id, id)
        ON DELETE CASCADE,
    CONSTRAINT meal_plan_participant_allocation_component_fk
        FOREIGN KEY (entry_id, component_id)
        REFERENCES meal_plan_component (entry_id, id)
        ON DELETE CASCADE,
    CONSTRAINT meal_plan_participant_allocation_unique
        UNIQUE (participant_id, component_id),
    CONSTRAINT meal_plan_participant_allocation_status_valid
        CHECK (status IN ('planned', 'eaten', 'not_eaten')),
    CONSTRAINT meal_plan_participant_allocation_kind_valid
        CHECK (allocated_kind IN ('measure', 'servings', 'packs')),
    CONSTRAINT meal_plan_participant_allocation_value_positive
        CHECK (allocated_value > 0),
    CONSTRAINT meal_plan_participant_allocation_unit_present
        CHECK ((allocated_kind = 'measure') = (allocated_unit IS NOT NULL)),
    CONSTRAINT meal_plan_participant_allocation_resolution_complete
        CHECK ((status = 'planned') = (resolved_by IS NULL AND resolved_at IS NULL)),
    CONSTRAINT meal_plan_participant_allocation_eaten_has_record
        CHECK (status <> 'eaten' OR consumption_record_id IS NOT NULL)
);

CREATE TABLE meal_guest_group (
    id          UUID PRIMARY KEY,
    entry_id    UUID NOT NULL REFERENCES meal_plan_entry (id) ON DELETE CASCADE,
    guest_count INTEGER NOT NULL,
    name        TEXT,
    note        TEXT,
    revision    BIGINT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT meal_guest_group_count_positive CHECK (guest_count > 0),
    CONSTRAINT meal_guest_group_entry_id_unique UNIQUE (entry_id, id)
);

CREATE TABLE meal_guest_allocation (
    id                UUID PRIMARY KEY,
    entry_id          UUID NOT NULL,
    guest_group_id    UUID NOT NULL,
    component_id      UUID NOT NULL,
    allocated_kind    TEXT NOT NULL,
    allocated_value   NUMERIC(16, 4) NOT NULL,
    allocated_unit    unit_code,
    status            TEXT NOT NULL DEFAULT 'planned',
    confirmed_kind    TEXT,
    confirmed_value   NUMERIC(16, 4),
    confirmed_unit    unit_code,
    resolved_by       UUID REFERENCES app_user (id) ON DELETE RESTRICT,
    resolved_at       TIMESTAMPTZ,

    CONSTRAINT meal_guest_allocation_group_fk
        FOREIGN KEY (entry_id, guest_group_id)
        REFERENCES meal_guest_group (entry_id, id)
        ON DELETE CASCADE,
    CONSTRAINT meal_guest_allocation_component_fk
        FOREIGN KEY (entry_id, component_id)
        REFERENCES meal_plan_component (entry_id, id)
        ON DELETE CASCADE,
    CONSTRAINT meal_guest_allocation_unique UNIQUE (guest_group_id, component_id),
    CONSTRAINT meal_guest_allocation_status_valid
        CHECK (status IN ('planned', 'eaten', 'not_eaten')),
    CONSTRAINT meal_guest_allocation_kind_valid
        CHECK (allocated_kind IN ('measure', 'servings', 'packs')),
    CONSTRAINT meal_guest_allocation_value_positive CHECK (allocated_value > 0),
    CONSTRAINT meal_guest_allocation_unit_present
        CHECK ((allocated_kind = 'measure') = (allocated_unit IS NOT NULL)),
    CONSTRAINT meal_guest_allocation_confirmed_complete
        CHECK (num_nonnulls(confirmed_kind, confirmed_value) <> 1),
    CONSTRAINT meal_guest_allocation_confirmed_kind_valid
        CHECK (confirmed_kind IS NULL OR confirmed_kind IN ('measure', 'servings', 'packs')),
    CONSTRAINT meal_guest_allocation_confirmed_value_positive
        CHECK (confirmed_value IS NULL OR confirmed_value > 0),
    CONSTRAINT meal_guest_allocation_resolution_complete
        CHECK ((status = 'planned') = (resolved_by IS NULL AND resolved_at IS NULL))
);

CREATE TABLE prepared_batch (
    id                      UUID PRIMARY KEY,

    recipe_id               UUID REFERENCES recipe (id) ON DELETE RESTRICT,
    meal_plan_entry_id      UUID REFERENCES meal_plan_entry (id) ON DELETE SET NULL,
    meal_plan_component_id  UUID REFERENCES meal_plan_component (id) ON DELETE SET NULL,

    prepared_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    servings_produced       NUMERIC(16, 4) NOT NULL,

    frozen_item_name        TEXT NOT NULL,
    nutrition_basis_amount  NUMERIC(16, 4),
    nutrition_basis_unit    unit_code,
    energy_kcal             NUMERIC(12, 3),
    protein_g               NUMERIC(12, 3),
    carbohydrate_g          NUMERIC(12, 3),
    sugar_g                 NUMERIC(12, 3),
    fat_g                   NUMERIC(12, 3),
    saturated_fat_g         NUMERIC(12, 3),
    fibre_g                 NUMERIC(12, 3),
    salt_g                  NUMERIC(12, 3),
    cholesterol_mg          NUMERIC(12, 3),
    nutrition_extra         JSONB NOT NULL,
    nutrition_quality       TEXT NOT NULL,

    created_by              UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    revision                BIGINT NOT NULL DEFAULT 1,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT prepared_batch_name_not_blank
        CHECK (btrim(frozen_item_name) <> ''),
    CONSTRAINT prepared_batch_servings_positive
        CHECK (servings_produced > 0),
    CONSTRAINT prepared_batch_basis_complete
        CHECK (num_nonnulls(nutrition_basis_amount, nutrition_basis_unit) <> 1),
    CONSTRAINT prepared_batch_basis_positive
        CHECK (nutrition_basis_amount IS NULL OR nutrition_basis_amount > 0),
    CONSTRAINT prepared_batch_extra_is_object
        CHECK (jsonb_typeof(nutrition_extra) = 'object'),
    CONSTRAINT prepared_batch_quality_valid
        CHECK (nutrition_quality IN ('known', 'estimated', 'partial', 'unknown')),
    CONSTRAINT prepared_batch_energy_non_negative
        CHECK (energy_kcal IS NULL OR energy_kcal >= 0),
    CONSTRAINT prepared_batch_protein_non_negative
        CHECK (protein_g IS NULL OR protein_g >= 0),
    CONSTRAINT prepared_batch_carbohydrate_non_negative
        CHECK (carbohydrate_g IS NULL OR carbohydrate_g >= 0),
    CONSTRAINT prepared_batch_sugar_non_negative
        CHECK (sugar_g IS NULL OR sugar_g >= 0),
    CONSTRAINT prepared_batch_fat_non_negative
        CHECK (fat_g IS NULL OR fat_g >= 0),
    CONSTRAINT prepared_batch_saturated_fat_non_negative
        CHECK (saturated_fat_g IS NULL OR saturated_fat_g >= 0),
    CONSTRAINT prepared_batch_fibre_non_negative
        CHECK (fibre_g IS NULL OR fibre_g >= 0),
    CONSTRAINT prepared_batch_salt_non_negative
        CHECK (salt_g IS NULL OR salt_g >= 0),
    CONSTRAINT prepared_batch_cholesterol_non_negative
        CHECK (cholesterol_mg IS NULL OR cholesterol_mg >= 0)
);

CREATE TABLE stock_item (
    id                UUID PRIMARY KEY,
    product_id        UUID REFERENCES product (id) ON DELETE RESTRICT,
    prepared_batch_id UUID REFERENCES prepared_batch (id) ON DELETE RESTRICT,

    tracking_mode     TEXT NOT NULL,
    quantity_value    NUMERIC(16, 4),
    quantity_unit     unit_code,

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

    CONSTRAINT stock_item_subject_exclusive
        CHECK (num_nonnulls(product_id, prepared_batch_id) = 1),
    CONSTRAINT stock_item_tracking_mode_valid
        CHECK (tracking_mode IN ('exact', 'estimated', 'not_tracked')),
    CONSTRAINT stock_item_storage_location_valid
        CHECK (storage_location IN ('ambient', 'chilled', 'frozen')),
    CONSTRAINT stock_item_quantity_matches_mode
        CHECK ((tracking_mode <> 'not_tracked')
            = (quantity_value IS NOT NULL AND quantity_unit IS NOT NULL)),
    CONSTRAINT stock_item_quantity_non_negative
        CHECK (quantity_value IS NULL OR quantity_value >= 0),
    CONSTRAINT stock_item_source_date_kind_valid
        CHECK (source_date_kind IS NULL OR source_date_kind IN ('use_by', 'best_before'))
);

CREATE TABLE stock_event (
    id                UUID PRIMARY KEY,
    stock_item_id     UUID NOT NULL REFERENCES stock_item (id) ON DELETE RESTRICT,
    source_kind       TEXT,
    source_id         UUID,
    source_label      TEXT,
    reverses_event_id UUID REFERENCES stock_event (id) ON DELETE SET NULL,

    event_kind        TEXT NOT NULL,
    quantity_delta    NUMERIC(16, 4),
    quantity_unit     unit_code,

    actor_user_id     UUID REFERENCES app_user (id) ON DELETE SET NULL,
    subject_member_id UUID REFERENCES household_member (id) ON DELETE SET NULL,

    note              TEXT,
    occurred_at       TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT stock_event_kind_valid
        CHECK (event_kind IN ('added', 'consumed', 'released', 'discarded', 'corrected',
                              'observed', 'moved', 'mode_changed', 'archived')),
    CONSTRAINT stock_event_source_kind_valid
        CHECK (source_kind IS NULL OR source_kind IN
               ('meal_plan_component', 'consumption_record', 'purchase', 'prepared_batch'))
);

CREATE TABLE stock_effect (
    id                UUID PRIMARY KEY,

    source_kind       TEXT NOT NULL,
    source_id         UUID NOT NULL,
    source_detail_id  UUID,

    stock_item_id     UUID NOT NULL REFERENCES stock_item (id) ON DELETE RESTRICT,
    product_id        UUID REFERENCES product (id) ON DELETE RESTRICT,
    prepared_batch_id UUID REFERENCES prepared_batch (id) ON DELETE RESTRICT,

    state             TEXT NOT NULL DEFAULT 'applied',

    applied_mode      TEXT NOT NULL,
    applied_unit      unit_code NOT NULL,
    exact_delta       NUMERIC(16, 4),
    estimated_delta   NUMERIC(16, 4),
    requested_value   NUMERIC(16, 4) NOT NULL,

    apply_event_id    UUID NOT NULL,
    applied_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    released_at       TIMESTAMPTZ,
    note              TEXT,

    CONSTRAINT stock_effect_source_kind_valid
        CHECK (source_kind IN
               ('meal_plan_component', 'consumption_record', 'purchase', 'prepared_batch')),
    CONSTRAINT stock_effect_subject_exclusive
        CHECK (num_nonnulls(product_id, prepared_batch_id) = 1),
    CONSTRAINT stock_effect_state_valid
        CHECK (state IN ('applied', 'released', 'release_failed')),
    CONSTRAINT stock_effect_mode_valid
        CHECK (applied_mode IN ('exact', 'estimated')),
    CONSTRAINT stock_effect_exact_delta_matches_mode
        CHECK ((applied_mode = 'exact') = (exact_delta IS NOT NULL)),
    CONSTRAINT stock_effect_estimated_delta_matches_mode
        CHECK ((applied_mode = 'estimated') = (estimated_delta IS NOT NULL))
);

CREATE TABLE shopping_cadence (
    singleton       BOOLEAN PRIMARY KEY DEFAULT TRUE,

    interval_weeks  INTEGER NOT NULL,
    days_of_week    SMALLINT[] NOT NULL,
    anchor_date     DATE NOT NULL,
    usual_time      TIME,

    revision        BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT shopping_cadence_singleton
        CHECK (singleton),
    CONSTRAINT shopping_cadence_interval_valid
        CHECK (interval_weeks BETWEEN 1 AND 8),
    CONSTRAINT shopping_cadence_days_valid
        CHECK (cardinality(days_of_week) BETWEEN 1 AND 7
               AND days_of_week <@ ARRAY[1, 2, 3, 4, 5, 6, 7]::SMALLINT[])
);

CREATE TABLE shopping_opportunity (
    id              UUID PRIMARY KEY,

    generated_for   DATE,
    effective_date  DATE,
    usual_time      TIME,
    state           TEXT NOT NULL,
    note            TEXT,

    revision        BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT shopping_opportunity_state_valid
        CHECK (state IN ('moved', 'skipped', 'one_off')),
    CONSTRAINT shopping_opportunity_one_off_has_no_origin
        CHECK ((state = 'one_off') = (generated_for IS NULL)),
    CONSTRAINT shopping_opportunity_skipped_has_no_date
        CHECK ((state = 'skipped') = (effective_date IS NULL))
);

CREATE TABLE purchase (
    id                UUID PRIMARY KEY,

    ingredient_id     UUID REFERENCES ingredient (id) ON DELETE RESTRICT,
    product_id        UUID REFERENCES product (id) ON DELETE RESTRICT,
    prepared_meal_id  UUID REFERENCES prepared_meal (id) ON DELETE RESTRICT,
    name              TEXT,
    quantity_value    NUMERIC(16, 4),
    quantity_unit     unit_code,

    opportunity_date  DATE,
    state             TEXT NOT NULL,
    stock_item_id     UUID REFERENCES stock_item (id) ON DELETE SET NULL,

    purchased_at      TIMESTAMPTZ NOT NULL,
    actor_user_id     UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    note              TEXT,

    revision          BIGINT NOT NULL DEFAULT 1,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT purchase_has_a_subject
        CHECK (
            num_nonnulls(ingredient_id, prepared_meal_id, product_id) >= 1
            OR btrim(coalesce(name, '')) <> ''
        ),
    CONSTRAINT purchase_state_valid
        CHECK (state IN ('pending', 'reconciled', 'cancelled')),
    CONSTRAINT purchase_reconciled_has_stock
        CHECK ((state = 'reconciled') = (stock_item_id IS NOT NULL)),
    CONSTRAINT purchase_quantity_complete
        CHECK ((quantity_value IS NULL) = (quantity_unit IS NULL)),
    CONSTRAINT purchase_quantity_positive
        CHECK (quantity_value IS NULL OR quantity_value > 0)
);

CREATE TABLE shopping_list_item (
    id                UUID PRIMARY KEY,

    ingredient_id     UUID REFERENCES ingredient (id) ON DELETE CASCADE,
    product_id        UUID REFERENCES product (id) ON DELETE CASCADE,
    prepared_meal_id  UUID REFERENCES prepared_meal (id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    quantity_value    NUMERIC(16, 4),
    quantity_unit     unit_code,

    section           shopping_section_code,
    opportunity_date  DATE,

    created_by        UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,
    revision          BIGINT NOT NULL DEFAULT 1,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT shopping_list_item_name_not_blank
        CHECK (btrim(name) <> ''),
    CONSTRAINT shopping_list_item_quantity_complete
        CHECK ((quantity_value IS NULL) = (quantity_unit IS NULL)),
    CONSTRAINT shopping_list_item_quantity_positive
        CHECK (quantity_value IS NULL OR quantity_value > 0)
);

CREATE TABLE shopping_trip (
    id                UUID PRIMARY KEY,

    opportunity_date  DATE NOT NULL,
    state             TEXT NOT NULL,
    started_at        TIMESTAMPTZ NOT NULL,
    finished_at       TIMESTAMPTZ,
    started_by        UUID NOT NULL REFERENCES app_user (id) ON DELETE RESTRICT,

    revision          BIGINT NOT NULL DEFAULT 1,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT shopping_trip_state_valid
        CHECK (state IN ('shopping', 'finished')),
    CONSTRAINT shopping_trip_finished_has_a_time
        CHECK ((state = 'finished') = (finished_at IS NOT NULL))
);

CREATE TABLE shopping_trip_row (
    id                UUID PRIMARY KEY,
    trip_id           UUID NOT NULL REFERENCES shopping_trip (id) ON DELETE CASCADE,

    ingredient_id     UUID REFERENCES ingredient (id) ON DELETE CASCADE,
    product_id        UUID REFERENCES product (id) ON DELETE CASCADE,
    prepared_meal_id  UUID REFERENCES prepared_meal (id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    quantity_value    NUMERIC(16, 4),
    quantity_unit     unit_code,
    section           shopping_section_code,
    position          INTEGER NOT NULL,

    CONSTRAINT shopping_trip_row_name_not_blank
        CHECK (btrim(name) <> ''),
    CONSTRAINT shopping_trip_row_quantity_complete
        CHECK ((quantity_value IS NULL) = (quantity_unit IS NULL))
);

CREATE TABLE shopping_suggestion_dismissal (
    opportunity_date  DATE NOT NULL,
    ingredient_id     UUID REFERENCES ingredient (id) ON DELETE CASCADE,
    product_id        UUID REFERENCES product (id) ON DELETE CASCADE,
    prepared_meal_id  UUID REFERENCES prepared_meal (id) ON DELETE CASCADE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT shopping_suggestion_dismissal_one_subject
        CHECK (num_nonnulls(ingredient_id, prepared_meal_id, product_id) = 1)
);

CREATE UNIQUE INDEX shopping_suggestion_dismissal_ingredient
    ON shopping_suggestion_dismissal (opportunity_date, ingredient_id)
    WHERE ingredient_id IS NOT NULL;
CREATE UNIQUE INDEX shopping_suggestion_dismissal_product
    ON shopping_suggestion_dismissal (opportunity_date, product_id)
    WHERE product_id IS NOT NULL;
CREATE UNIQUE INDEX shopping_suggestion_dismissal_prepared_meal
    ON shopping_suggestion_dismissal (opportunity_date, prepared_meal_id)
    WHERE prepared_meal_id IS NOT NULL;

CREATE TABLE weight_record (
    id           UUID PRIMARY KEY,
    member_id    UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    weight_kg    NUMERIC(6, 3) NOT NULL,
    recorded_on  DATE NOT NULL,
    recorded_at  TIMESTAMPTZ,
    source       TEXT NOT NULL,
    recorded_by  UUID REFERENCES app_user (id) ON DELETE SET NULL,

    revision     BIGINT NOT NULL DEFAULT 1,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT weight_record_source_known
        CHECK (source IN ('manual', 'health_connect')),
    CONSTRAINT weight_record_weight_plausible
        CHECK (weight_kg > 0 AND weight_kg <= 635)
);

CREATE TABLE weight_goal (
    id                       UUID PRIMARY KEY,
    member_id                UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    objective                TEXT NOT NULL,
    starting_weight_kg       NUMERIC(6, 3) NOT NULL,
    target_weight_kg         NUMERIC(6, 3),
    planned_rate_kg_per_week NUMERIC(5, 3),
    started_on               DATE NOT NULL,

    revision                 BIGINT NOT NULL DEFAULT 1,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT weight_goal_objective_known
        CHECK (objective IN ('lose', 'maintain', 'gain')),
    CONSTRAINT weight_goal_weights_plausible
        CHECK (
            starting_weight_kg > 0 AND starting_weight_kg <= 635
            AND (target_weight_kg IS NULL OR (target_weight_kg > 0 AND target_weight_kg <= 635))
        ),
    CONSTRAINT weight_goal_directed
        CHECK (
            (objective = 'maintain'
                AND target_weight_kg IS NULL
                AND planned_rate_kg_per_week IS NULL)
            OR (objective = 'lose'
                AND target_weight_kg IS NOT NULL
                AND target_weight_kg < starting_weight_kg
                AND planned_rate_kg_per_week IS NOT NULL
                AND planned_rate_kg_per_week >= 0)
            OR (objective = 'gain'
                AND target_weight_kg IS NOT NULL
                AND target_weight_kg > starting_weight_kg
                AND planned_rate_kg_per_week IS NOT NULL
                AND planned_rate_kg_per_week > 0)
        )
);

CREATE TABLE member_body_profile (
    member_id          UUID PRIMARY KEY REFERENCES household_member (id) ON DELETE CASCADE,
    date_of_birth      DATE,
    sex                TEXT,
    height_cm          NUMERIC(5, 1),
    habitual_activity  TEXT,
    revision           BIGINT NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT member_body_profile_sex_known
        CHECK (sex IS NULL OR sex IN ('male', 'female')),
    CONSTRAINT member_body_profile_activity_known
        CHECK (habitual_activity IS NULL OR habitual_activity IN
            ('mostly_sedentary', 'lightly_active', 'active', 'very_active')),
    CONSTRAINT member_body_profile_height_plausible
        CHECK (height_cm IS NULL OR (height_cm > 50 AND height_cm <= 260))
);

CREATE TABLE calorie_target_calculation (
    id                          UUID PRIMARY KEY,
    member_id                   UUID NOT NULL REFERENCES household_member (id) ON DELETE CASCADE,
    nutrition_target_id         UUID NOT NULL REFERENCES nutrition_target (id) ON DELETE CASCADE,
    calculated_on               DATE NOT NULL,
    formula                     TEXT NOT NULL,
    activity_source             TEXT NOT NULL,
    habitual_activity           TEXT NOT NULL,
    age_years                   INTEGER NOT NULL,
    sex                         TEXT NOT NULL,
    height_cm                   NUMERIC(5, 1) NOT NULL,
    weight_kg                   NUMERIC(6, 3) NOT NULL,
    objective                   TEXT NOT NULL,
    emphasis                    TEXT NOT NULL DEFAULT 'general',
    requested_rate_kg_per_week  NUMERIC(5, 3),
    applied_rate_kg_per_week    NUMERIC(5, 3),
    maintenance_kcal            NUMERIC(12, 3) NOT NULL,
    adjustment_kcal             NUMERIC(12, 3) NOT NULL,
    recommended_kcal            NUMERIC(12, 3) NOT NULL,
    floor_kcal                  NUMERIC(12, 3) NOT NULL,
    eased                       BOOLEAN NOT NULL DEFAULT false,
    revision                    BIGINT NOT NULL DEFAULT 1,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT calorie_target_calculation_emphasis_known
        CHECK (emphasis IN ('general', 'muscle', 'endurance'))
);

CREATE UNIQUE INDEX calorie_target_calculation_target_unique
    ON calorie_target_calculation (nutrition_target_id);

CREATE TABLE meal_template (
    id            UUID PRIMARY KEY,
    owner_id      UUID NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    name          TEXT NOT NULL,

    revision      BIGINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at   TIMESTAMPTZ,

    CONSTRAINT meal_template_name_not_blank
        CHECK (btrim(name) <> '')
);

CREATE TABLE meal_template_component (
    id                UUID PRIMARY KEY,
    template_id       UUID NOT NULL REFERENCES meal_template (id) ON DELETE CASCADE,
    position          INTEGER NOT NULL,

    item_kind         TEXT NOT NULL,
    product_id        UUID REFERENCES product (id) ON DELETE RESTRICT,
    recipe_id         UUID REFERENCES recipe (id) ON DELETE RESTRICT,
    ingredient_id     UUID REFERENCES ingredient (id) ON DELETE RESTRICT,
    prepared_meal_id  UUID REFERENCES prepared_meal (id) ON DELETE RESTRICT,

    amount_kind       TEXT NOT NULL,
    amount_value      NUMERIC(16, 4) NOT NULL,
    amount_unit       unit_code,

    CONSTRAINT meal_template_component_position_non_negative
        CHECK (position >= 0),
    CONSTRAINT meal_template_component_item_kind_valid
        CHECK (item_kind IN ('product', 'recipe', 'ingredient', 'prepared_meal')),
    CONSTRAINT meal_template_component_item_ref_exclusive
        CHECK (
            num_nonnulls(product_id, recipe_id, ingredient_id, prepared_meal_id) = 1
            AND (item_kind = 'product')       = (product_id IS NOT NULL)
            AND (item_kind = 'recipe')        = (recipe_id IS NOT NULL)
            AND (item_kind = 'ingredient')    = (ingredient_id IS NOT NULL)
            AND (item_kind = 'prepared_meal') = (prepared_meal_id IS NOT NULL)
        ),
    CONSTRAINT meal_template_component_amount_kind_valid
        CHECK (amount_kind IN ('measure', 'servings', 'packs')),
    CONSTRAINT meal_template_component_amount_value_positive
        CHECK (amount_value > 0),
    CONSTRAINT meal_template_component_amount_unit_present
        CHECK ((amount_kind = 'measure') = (amount_unit IS NOT NULL)),
    CONSTRAINT meal_template_component_template_id_position_unique UNIQUE (template_id, position)
);

INSERT INTO household_settings (singleton, breakfast_time, lunch_time, dinner_time)
VALUES (TRUE, '08:00', '12:30', '18:00');

CREATE UNIQUE INDEX ingredient_name_unique ON ingredient (lower(name));

CREATE UNIQUE INDEX ingredient_seed_key_unique ON ingredient (seed_key) WHERE seed_key IS NOT NULL;

CREATE INDEX ingredient_name_trgm ON ingredient USING gin (name gin_trgm_ops);

CREATE UNIQUE INDEX product_barcode_unique ON product (barcode) WHERE barcode IS NOT NULL;

CREATE UNIQUE INDEX product_seed_key_unique ON product (seed_key) WHERE seed_key IS NOT NULL;

CREATE INDEX product_name_trgm ON product USING gin (name gin_trgm_ops);

CREATE INDEX product_mapped_ingredient ON product (mapped_ingredient_id)
    WHERE mapped_ingredient_id IS NOT NULL;

CREATE UNIQUE INDEX app_user_username_unique ON app_user (lower(username));

CREATE UNIQUE INDEX app_user_auth_subject_unique ON app_user (auth_subject)
    WHERE auth_subject IS NOT NULL;

CREATE UNIQUE INDEX household_member_display_name_unique
    ON household_member (lower(display_name));

CREATE INDEX member_access_grant_subject ON member_access_grant (subject_member_id);

CREATE INDEX consumption_record_member_day ON consumption_record (member_id, consumed_on);

CREATE INDEX consumption_record_product ON consumption_record (product_id);

CREATE INDEX meal_plan_entry_occasion ON meal_plan_entry (occasion_id);

CREATE INDEX meal_occasion_absence_member ON meal_occasion_absence (member_id);

CREATE INDEX meal_plan_component_entry ON meal_plan_component (entry_id, position);

CREATE INDEX meal_plan_component_product ON meal_plan_component (product_id);

CREATE INDEX meal_plan_component_recipe ON meal_plan_component (recipe_id);

CREATE UNIQUE INDEX consumption_record_meal_plan_component_member_unique
    ON consumption_record (meal_plan_component_id, member_id)
    WHERE meal_plan_component_id IS NOT NULL;

CREATE UNIQUE INDEX nutrition_target_member_effective_from_unique
    ON nutrition_target (member_id, effective_from);

CREATE INDEX recipe_owner ON recipe (owner_id, archived_at);

CREATE INDEX recipe_name_trgm ON recipe USING gin (name gin_trgm_ops);

CREATE INDEX recipe_component_recipe ON recipe_component (recipe_id, position);

CREATE INDEX recipe_component_product ON recipe_component (product_id) WHERE product_id IS NOT NULL;

CREATE INDEX recipe_component_ingredient ON recipe_component (ingredient_id) WHERE ingredient_id IS NOT NULL;

CREATE INDEX recipe_instruction_recipe ON recipe_instruction (recipe_id, position);

CREATE UNIQUE INDEX recipe_tag_case_insensitive
    ON recipe_tag (recipe_id, lower(tag));

CREATE INDEX consumption_record_recipe ON consumption_record (recipe_id);

CREATE INDEX stock_item_product ON stock_item (product_id);

CREATE INDEX stock_item_usability_deadline ON stock_item (usability_deadline)
    WHERE usability_deadline IS NOT NULL AND archived_at IS NULL;

CREATE INDEX stock_event_item ON stock_event (stock_item_id, occurred_at DESC);

CREATE INDEX meal_plan_participant_entry ON meal_plan_participant (entry_id);

CREATE INDEX meal_plan_participant_member ON meal_plan_participant (member_id);

CREATE INDEX meal_plan_participant_allocation_participant
    ON meal_plan_participant_allocation (participant_id);

CREATE INDEX meal_plan_participant_allocation_component
    ON meal_plan_participant_allocation (component_id);

CREATE INDEX meal_guest_group_entry ON meal_guest_group (entry_id);

CREATE INDEX meal_guest_allocation_group ON meal_guest_allocation (guest_group_id);

CREATE INDEX meal_guest_allocation_component ON meal_guest_allocation (component_id);

CREATE UNIQUE INDEX stock_effect_active_source_item_unique
    ON stock_effect (source_kind, source_id, source_detail_id, stock_item_id)
    NULLS NOT DISTINCT
    WHERE state = 'applied';

CREATE INDEX stock_effect_source ON stock_effect (source_kind, source_id);

CREATE INDEX stock_effect_item ON stock_effect (stock_item_id);

CREATE INDEX consumption_record_meal_plan_entry
    ON consumption_record (meal_plan_entry_id)
    WHERE meal_plan_entry_id IS NOT NULL;

CREATE UNIQUE INDEX shopping_opportunity_generated_for_unique
    ON shopping_opportunity (generated_for)
    WHERE generated_for IS NOT NULL;

CREATE INDEX shopping_opportunity_effective_date
    ON shopping_opportunity (effective_date)
    WHERE effective_date IS NOT NULL;

CREATE INDEX purchase_state ON purchase (state);

CREATE INDEX purchase_opportunity_date ON purchase (opportunity_date)
    WHERE opportunity_date IS NOT NULL;

CREATE INDEX shopping_list_item_opportunity_date ON shopping_list_item (opportunity_date)
    WHERE opportunity_date IS NOT NULL;

CREATE UNIQUE INDEX shopping_trip_one_per_shop ON shopping_trip (opportunity_date);

CREATE INDEX shopping_trip_row_trip ON shopping_trip_row (trip_id);

CREATE INDEX weight_record_member_recorded
    ON weight_record (member_id, recorded_on DESC, recorded_at DESC NULLS LAST);

CREATE UNIQUE INDEX weight_goal_member_unique ON weight_goal (member_id);

CREATE INDEX prepared_batch_recipe ON prepared_batch (recipe_id)
    WHERE recipe_id IS NOT NULL;

CREATE INDEX prepared_batch_component ON prepared_batch (meal_plan_component_id)
    WHERE meal_plan_component_id IS NOT NULL;

CREATE INDEX stock_item_prepared_batch ON stock_item (prepared_batch_id)
    WHERE prepared_batch_id IS NOT NULL;

CREATE UNIQUE INDEX prepared_meal_name_unique ON prepared_meal (lower(name));

CREATE UNIQUE INDEX prepared_meal_seed_key_unique ON prepared_meal (seed_key) WHERE seed_key IS NOT NULL;

CREATE INDEX prepared_meal_name_trgm ON prepared_meal USING gin (name gin_trgm_ops);

CREATE INDEX product_mapped_prepared_meal ON product (mapped_prepared_meal_id)
    WHERE mapped_prepared_meal_id IS NOT NULL;

CREATE INDEX meal_plan_component_ingredient ON meal_plan_component (ingredient_id)
    WHERE ingredient_id IS NOT NULL;

CREATE INDEX meal_plan_component_prepared_meal ON meal_plan_component (prepared_meal_id)
    WHERE prepared_meal_id IS NOT NULL;

CREATE INDEX consumption_record_ingredient ON consumption_record (ingredient_id)
    WHERE ingredient_id IS NOT NULL;

CREATE INDEX consumption_record_prepared_meal ON consumption_record (prepared_meal_id)
    WHERE prepared_meal_id IS NOT NULL;

CREATE INDEX meal_template_owner_name_trgm ON meal_template USING gin (name gin_trgm_ops);

CREATE INDEX meal_template_component_template ON meal_template_component (template_id, position);

CREATE INDEX meal_template_owner ON meal_template (owner_id);

CREATE INDEX ingredient_created_by ON ingredient (created_by);

CREATE INDEX ingredient_updated_by ON ingredient (updated_by);

CREATE INDEX product_created_by ON product (created_by);

CREATE INDEX product_updated_by ON product (updated_by);

CREATE INDEX member_access_grant_granted_by ON member_access_grant (granted_by);

CREATE INDEX consumption_record_recorded_by ON consumption_record (recorded_by);

CREATE INDEX meal_plan_entry_created_by ON meal_plan_entry (created_by);

CREATE INDEX meal_plan_entry_updated_by ON meal_plan_entry (updated_by);

CREATE INDEX meal_occasion_created_by ON meal_occasion (created_by);

CREATE INDEX meal_occasion_updated_by ON meal_occasion (updated_by);

CREATE INDEX meal_occasion_absence_created_by ON meal_occasion_absence (created_by);

CREATE INDEX recipe_created_by ON recipe (created_by);

CREATE INDEX recipe_updated_by ON recipe (updated_by);

CREATE INDEX stock_event_actor_user_id ON stock_event (actor_user_id);

CREATE INDEX meal_plan_participant_allocation_resolved_by ON meal_plan_participant_allocation (resolved_by);

CREATE INDEX meal_guest_allocation_resolved_by ON meal_guest_allocation (resolved_by);

CREATE INDEX prepared_batch_created_by ON prepared_batch (created_by);

CREATE INDEX purchase_actor_user_id ON purchase (actor_user_id);

CREATE INDEX shopping_list_item_created_by ON shopping_list_item (created_by);

CREATE INDEX weight_record_recorded_by ON weight_record (recorded_by);

CREATE INDEX purchase_ingredient_id ON purchase (ingredient_id);

CREATE INDEX purchase_product_id ON purchase (product_id);

CREATE INDEX purchase_stock_item_id ON purchase (stock_item_id);

CREATE INDEX purchase_prepared_meal_id ON purchase (prepared_meal_id);

CREATE INDEX shopping_list_item_ingredient_id ON shopping_list_item (ingredient_id);

CREATE INDEX shopping_list_item_product_id ON shopping_list_item (product_id);

CREATE INDEX shopping_list_item_prepared_meal_id ON shopping_list_item (prepared_meal_id);

CREATE INDEX shopping_trip_row_ingredient_id ON shopping_trip_row (ingredient_id);

CREATE INDEX shopping_trip_row_product_id ON shopping_trip_row (product_id);

CREATE INDEX shopping_trip_row_prepared_meal_id ON shopping_trip_row (prepared_meal_id);

CREATE INDEX meal_template_component_product_id ON meal_template_component (product_id);

CREATE INDEX meal_template_component_recipe_id ON meal_template_component (recipe_id);

CREATE INDEX meal_template_component_ingredient_id ON meal_template_component (ingredient_id);

CREATE INDEX meal_template_component_prepared_meal_id ON meal_template_component (prepared_meal_id);

CREATE INDEX stock_effect_product_id ON stock_effect (product_id);

CREATE INDEX stock_effect_prepared_batch_id ON stock_effect (prepared_batch_id);

CREATE INDEX prepared_batch_meal_plan_entry_id ON prepared_batch (meal_plan_entry_id);

CREATE INDEX meal_plan_participant_allocation_entry_id ON meal_plan_participant_allocation (entry_id);

CREATE INDEX meal_guest_allocation_entry_id ON meal_guest_allocation (entry_id);
