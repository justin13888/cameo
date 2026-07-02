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

rename_id | fix_vote_average | fix_bool_defaults | strip_examples
