# Deterministic normalizations applied to the raw TMDB OpenAPI 3.1 spec so that
# spargen (`cargo xtask codegen`) can generate a client without any manual
# post-editing. None of them depends on the generator: each corrects the spec
# itself.
#
# Each transform targets a specific, recurring upstream defect. Keep this file
# in sync with the "Known Generated-Type Quirks" table in DEVELOPMENT.md.

# (1) Rename inline `_id` -> `mongo_id`.
#     The TV-season-details response embeds a MongoDB ObjectId as `_id`, which
#     collides with the sibling `id` field once the leading underscore is
#     dropped from the Rust field name.
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
#     booleans. An upstream defect: a string default on a boolean schema does
#     not validate against its own type.
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
#     none nullable or `required`. A generator may turn a non-required property
#     with a default into a plain `T` + `#[serde(default)]`, which only covers a
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

# (6) Mark every optional response property nullable (`"type": [T, "null"]`).
#     OpenAPI 3.1 expresses nullability in the type, and the spec marks no
#     response property nullable, yet TMDB sends an explicit `null` for many of
#     them (`id`, `vote_count`, `in_production`, image paths, ...). spargen
#     decodes a non-nullable optional field strictly, so a single `null` would
#     fail the whole response; a nullable optional field reads `null` and an
#     absent key alike as `None`. Properties a schema lists in `required`, and
#     schemas with no plain string `type` (`{}` accepts null already), are left
#     as they are. Request bodies are not touched.
def nullable_response_properties:
  walk(if type == "object" and (.properties | type) == "object"
       then (.required // []) as $required
         | .properties |= with_entries(
             if (.key | IN($required[]) | not)
                and (.value | type) == "object"
                and (.value.type | type) == "string"
             then .value.type = [.value.type, "null"]
             else . end)
       else . end);
def nullable_responses:
  .paths |= map_values(map_values(
    if type == "object" and has("responses")
    then .responses |= nullable_response_properties
    else . end));

# (7) Name the inline schemas the client uses.
#     Every TMDB schema is inline, and spargen names an inline schema by a hash
#     of its JSON Pointer (`ResponseBodyC0e107d6`). Hoisting each operation's
#     `200` JSON body into `components/schemas/<OperationId>Response`, and each
#     enum parameter schema into `<OperationId><ParamName>`, gives the
#     generated types stable, readable names (`MovieDetailsResponse`,
#     `MovieDetailsResponseGenresItem`, `TrendingMoviesTimeWindow`). The
#     schemas themselves are unchanged.
def pascal_case(sep): split(sep) | map((.[0:1] | ascii_upcase) + .[1:]) | join("");
def hoist_schema($pointer; $name):
  getpath($pointer) as $schema
  | if ($schema | type) == "object" and ($schema | has("$ref") | not)
    then .components.schemas[$name] = $schema
      | setpath($pointer; {"$ref": ("#/components/schemas/" + $name)})
    else . end;
def name_schemas:
  reduce (.paths | to_entries[] | .key as $path | .value | to_entries[]
          | select((.value | type) == "object" and (.value | has("operationId")))
          | {path: $path, method: .key, id: (.value.operationId | pascal_case("-")),
             params: (.value.parameters // [])}) as $op (.;
    hoist_schema(["paths", $op.path, $op.method, "responses", "200", "content",
                  "application/json", "schema"]; $op.id + "Response")
    | reduce range(0; $op.params | length) as $i (.;
        if ($op.params[$i].schema.enum | type) == "array"
        then hoist_schema(["paths", $op.path, $op.method, "parameters", $i, "schema"];
                          $op.id + ($op.params[$i].name | pascal_case("_")))
        else . end));

# (8) Declare both TMDB credentials as security schemes.
#     The spec declares one scheme, an `apiKey` named `Authorization` in the
#     header, which is how it spells the v4 bearer token. It omits the v3 API
#     key, which TMDB accepts as an `api_key` query parameter. Declaring the v4
#     token as `http` bearer and adding the v3 key as a query `apiKey`, with
#     either one satisfying every operation, lets the generated client attach
#     whichever credential `TmdbClient` registers (and keep it out of `Debug`
#     and of the `_url` accessors).
def declare_security:
  .components.securitySchemes = {
    "v4_bearer": {
      "type": "http",
      "scheme": "bearer",
      "description": "TMDB v4 API Read Access Token"
    },
    "v3_api_key": {
      "type": "apiKey",
      "in": "query",
      "name": "api_key",
      "description": "TMDB v3 API key"
    }
  }
  | .security = [{"v4_bearer": []}, {"v3_api_key": []}];

# (9) Give the movie-recommendations response its real shape.
#     The spec declares it as an object with no properties, which generates a
#     struct that discards the whole body. TMDB returns the same paged movie
#     list as the similar-movies endpoint, so point it at that schema. Applied
#     after (7) and only while the upstream schema is still empty.
def fix_movie_recommendations:
  ["paths", "/3/movie/{movie_id}/recommendations", "get", "responses", "200", "content",
   "application/json", "schema"] as $pointer
  | if ((.components.schemas.MovieRecommendationsResponse.properties // {}) | length) == 0
       and .components.schemas.MovieSimilarResponse != null
    then setpath($pointer; {"$ref": "#/components/schemas/MovieSimilarResponse"})
      | del(.components.schemas.MovieRecommendationsResponse)
    else . end;

rename_id | fix_vote_average | fix_bool_defaults | strip_examples | strip_property_defaults
| nullable_responses | name_schemas | fix_movie_recommendations | declare_security
