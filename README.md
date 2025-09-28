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

**Example:**

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
