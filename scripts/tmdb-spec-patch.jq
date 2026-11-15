# Deterministic normalizations applied to the raw TMDB OpenAPI spec so that
# progenitor can generate a client without any manual post-editing.
#
# Each transform targets a specific, recurring upstream defect. Keep this file
# in sync with the "Known Generated-Type Quirks" table in DEVELOPMENT.md.

# (1) Rename inline `_id` -> `mongo_id`.
#     The TV-season-details response embeds a MongoDB ObjectId as `_id`, which
#     collides with the sibling `id` field and makes progenitor emit a struct
#     with two fields named `id`.
def rename_id:
  walk(if type == "object" and has("_id")
       then (.mongo_id = ._id) | del(._id)
       else . end);

# (2) Coerce integer `vote_average` schemas to number.
#     Several TV list schemas declare `vote_average` as `integer`, but TMDB
#     returns floating-point averages at runtime.
def fix_vote_average:
  walk(if type == "object"
          and (.vote_average | type) == "object"
          and (.vote_average.type == "integer")
       then .vote_average.type = "number"
       else . end);

# (3) Coerce string-typed boolean defaults (e.g. `"default": "false"`) to real
#     booleans. An upstream defect that makes typify reject the schema with
#     `TypeError(InvalidValue)`.
def fix_bool_defaults:
  walk(if type == "object" and .type == "boolean" and (.default | type) == "string"
       then .default = (.default == "true")
       else . end);

# (4) Strip `example`/`examples` payloads.
#     These are ignored by code generation but account for ~75% of the raw
#     spec's size (and are emitted as doc-comment noise in the generated code).
def strip_examples:
  walk(if type == "object" then del(.example, .examples) else . end);

# (5) Drop `default` from object property schemas.
#     The spec gives most response fields a `default` (`0`, `true`) but marks
#     none `nullable` or `required`. typify turns a non-required property with
#     a default into a plain `T` + `#[serde(default)]`, which only covers a
#     *missing* key: TMDB regularly sends an explicit `null` (e.g. `id`,
#     `vote_count`, `in_production`), and that one null fails deserialization
#     of the whole response. Without the default the property becomes an
#     `Option<T>`, which accepts both absent and null. The `true` defaults were
#     also misleading (an unknown `adult`/`video`/`in_production` must not read
#     as `true`). Operation parameter schemas (`parameters[].schema`) are not
#     under `properties` and keep their defaults: they describe request-side
#     values, not response shapes.
def strip_property_defaults:
  walk(if type == "object" and (.properties | type) == "object"
       then .properties |= map_values(if type == "object" then del(.default) else . end)
       else . end);

rename_id | fix_vote_average | fix_bool_defaults | strip_examples | strip_property_defaults
