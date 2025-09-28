# Rule Evaluator

## Running

### Docker

The server can be run on Docker via the provider `Dockerfile` and `docker-compose.yaml`.

```
docker compose up --build
```

The server will be accessible on `localhost:8080`.

### Locally

The server can also be run locally directly via `cargo`

```
git clone <todo>
cd <todo>
cargo run
```

The server will be accessible on `localhost:8080`.

## Schema

### Predicate

A predicate can be either a raw predicate defining the simplest given condition or a compound predicate that consists of one or more predicates and a logical operator.

```typescript
type Predicate = RawPredicate | CompoundPredicate;
```

**Raw Predicate**

```typescript
type RawPredicate = {
  path: string;
  operator: Operator;
  value: Object;
};
```

- `path`: The path to the field being tested. Can be either a simple field name or multiple field names separated by dots for tested nested fields. (e.g. `applicant.income`)
- `operator`: The operator to use for the check, supports various operators such as `equal`, `greater`, `less`, `contains`. See the [Operators](#operators) section for a detailed breakdown of each operator.
- `value`: The value to compare against, can be arbitrary JSON.

**Compund Predicate**

A compund predicate applies logial operators to one or more predicates (either raw or compound) and can be used to build more compicated rules from smaller building blocks.

```typescript
type CompoundPredicate =
  { not: Predicate }
| { any: Predicate[] }
| { all: Predicate:[] }
| { none: Predicate[] }
```

- `not` - Inverts the result of the child predicate.
- `any` - Evalutes `true` if and only if at least one child predicate evaluted as `true` - i.e. logical OR.
- `all` - Evalutes `true` if and only if all child predicates evaluted as `true` - i.e. logical AND
- `none` - Evalutes `true` if and only if all child predicates evaluated `false` - i.e. logical NOR. Provided as a convenient shorthand for `{ "not": {"any": Predicate[] }}`

### Operators

- `equal` / `==` - Evaluates strict equality. Supports arbitrary JSON and will perform deep equality checks. Does not perform any kind of type coercion so can only evaluate to true if both the input and value types are equal.
- `notEqual` / `!=` - Evalutes strict inequality. Shorthand for wrapping `equal` in `not` so same restrictions as for `equal` apply.
- Mathematical operators - The input and value type must both be `number`.
  - `greater` / `>`
  - `less` / `<`
  - `greaterEqual` / `>=`
  - `lessEqual` / `<=`
- `contains` / `in` - Evaluates whether the given value is an element of the input. Input type must be `T[]`. Supports arbitrary JSON for the value being checked itself.

### Rule

A rule is defined by an id, an error message in the case of failure, and a predicate tree consisting of nested conditions.

```typescript
type Rule = {
  id: string;
  message: string;
  predicate: Predicate;
};
```

<details>

<summary>Example</summary>

```JSON
{
  "id": "waterpark-rule",
  "message": "You must be at least 5'2'' and over the age of 12 to use this water slide",
  "predicate": {
    "all": [
      {
        "path": "age",
        "operator": ">=",
        "value": 12
      },
      {
        "any": [
          {
            "path": "height.feet",
            "operator": ">",
            "value": "5"
          },
          {
            "all": [
              {
                "path": "height.feet",
                "operator": "=",
                "value": 5
              },
              {
                "path": "height.inches",
                "operator": ">=",
                "value": 2
              }
            ]
          }
        ]
      }
    ]
  }
}
```

</details>

## Sample

<details>

<summary>Rules</summary>

```json
[
  {
    "id": "waterpark_height_rule",
    "message": "You must be at least 5'2'' to use this water slide.",
    "predicate": {
      "any": [
        {
          "path": "height.feet",
          "operator": ">",
          "value": 5
        },
        {
          "all": [
            {
              "path": "height.feet",
              "operator": "==",
              "value": 5
            },
            {
              "path": "height.inches",
              "operator": ">=",
              "value": 2
            }
          ]
        }
      ]
    }
  },
  {
    "id": "waterpark_age_rule",
    "message": "You must be at least age 12 to use this water slide",
    "predicate": {
      "all": [
        {
          "path": "age",
          "operator": ">=",
          "value": 12
        }
      ]
    }
  }
]
```

</details>

### 1 - Pass

```
curl http://localhost:8080/evaluate?rules=waterpark_height_rule,waterpark_age_rule \
    -X POST \
    --header "Content-Type: application/json" \
    --data '
{
  "age": 24,
  "height": {
    "feet": 7,
    "inches": 10
  }
}
'
```

```
{
  "result": "PASS",
  "reasons": [
    {
      "rule": "waterpark_height_rule",
      "requirement": "You must be at least 5'2'' to use this water slide.",
      "evaluation": "PASS"
    },
    {
      "rule": "waterpark_age_rule",
      "requirement": "You must be at least age 12 to use this water slide",
      "evaluation": "PASS"
    }
  ]
}
```

### 2 - Fail

```
curl http://localhost:8080/evaluate?rules=waterpark_height_rule,waterpark_age_rule \
    -X POST \
    --header "Content-Type: application/json" \
    --data '
{
  "age": 11,
  "height": {
    "feet": 5,
    "inches": 10
  }
}
'
```

```
{
  "result": "FAIL",
  "reasons": [
    {
      "rule": "waterpark_height_rule",
      "requirement": "You must be at least 5'2'' to use this water slide.",
      "evaluation": "PASS"
    },
    {
      "rule": "waterpark_age_rule",
      "requirement": "You must be at least age 12 to use this water slide",
      "evaluation": "FAIL"
    }
  ]
}
```

## Notes

### Assumptions / Design Decisions

- `/evalute` takes the list of rules to apply in the `rules` query param.
  - Since one of the goals was for this endpoint to accept arbitrary JSON the decision was made to include the list of rules to run in the query params instead of having the body be a mix of rule definitions + nested JSON object for testing.

### Edge cases / unhappy path handling

- ✅ Type checking
  - ✅ Mathematical ordering operators (>, < <=, >=) error if either of the arguments aren't numbers
  - ✅ `contains` operator errors for non-arrays
- ⚠️ API Errors
  - ✅ Creating rule with id that already exists will error with 404 and JSON error
  - ✅ Trying to get / edit a rule that doesn't exist will error with 404 and JSON error
  - ✅ Type checking errors will suface as 400 JSON errors.
  - ❌ JSON deserialization errors aren't surfaced as JSON
  - ❌ Default 404 page doesn't return any body
- ⚠️ Handling of missing fields (this _kinda_ mirrors JS behavior of missing fields returning `undefined` and only erroring after but not by explicit design)
  - ❌ A missing field evaluates to `null` instead of erroring
  - ✅ Trying to access a field of `null` _will_ error properly. (e.g. `foo.bar` when `foo` does no exist)

### Future Work

- Dockerfile improvements - the current Docker setup is minimal and requires fetching all depencies and building from scratch each time. This could be improved with a caching layer, such as [cargo-chef](https://github.com/LukeMathWalker/cargo-chef) to allow incremental builds and significantly speed up start up times.
- Introduce a better system for error handling (potentially via middleware) to ensure all errors can be returned as JSON.
- `InMemRuleRepository` that handles rules is susceptible to lock poisoning in the event a panic occurs during the handling of a request, causing future requests to timeout. While errors are properly handeled via `Result`, panics could still be possible so this should be addressed for any extensive use.
- Logging - adding logging of requests/responses and rule evaluations would be useful for debugging and audits. Something like the `tracing` / `tracing_subscriber` crates would work well to output logs to a file / some logging service.
- Caching - if it's common for the same input to be evaluated multiple times caching might be useful to avoid recomputation when neither the rule nor the input have changed.
- Metrics - it would be useful to emit metrics (e.g. general counts, request latency) to a central system (e.g. Grafana / Prometheus setup) for observability to detect anomalies and find potential areas of improvements.
- General code improvements - there's some parts of the code that could be structured a little better for better separation. (e.g. `RuleRepository` probably shouldn't be doing the evaluation itself given it's just a wrapper over a db-esque interface). The repository interface and `InMemRuleRepository` could likely also be a little improved to avoid the repetitive String cloning in some places.
- More extensive tests - while the current tests do a good job of having coverage end to end from serialization / deserialization, rule evaluation, response bodies and status codes, there's none the less some gaps with coverage that should be improved. - e.g. endpoints outside of the API.

## Error Samples

<details>

<summary>Duplicate rule creation</summary>

```
curl http://localhost:8080/rules \
    -X POST \
    --header "Content-Type: application/json" \
    --data '
{
  "id": "some-rule",
  "message": "test",
  "predicate": {
    "any": []
  }
}
'
```

```
400 Bad Request


{
  "error": {
    "message": "a rule with id some-rule already exists"
  }
}
```

</details>

<details>

<summary>No existing id reference</summary>

Trying to reference an id that doesn't exist will error. (update / get).
This does not happen for delete as it's designed to be idempotent.

```
curl http://localhost:8080/rules/some-non-existing-id -X DELETE
```

```
404 Not Found

{
  "error": {
    "message": "a rule with id some-non-existing-id does not exist"
  }
}
```

</details>

<details>

<summary>Rule evaluation errors</summary>

Rules themselves enforce various requirements. e.g. ordering operators can only be applied on numbers.

```
curl http://localhost:8080/evaluate?rules=waterpark_height_rule \
    -X POST \
    --header "Content-Type: application/json" \
    --data '
{
  "age": 24,
  "height": {
    "feet": "7 feet",
    "inches": 10
  }
}
'
```

```
404 Bad Request

{
  "error": {
    "message": "failed to evaluate rule waterpark_height_rule: cannot compare string with number using operator Greater"
  }
}
```

</details>
