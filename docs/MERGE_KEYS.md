# YAML Merge Key Support

This library fully supports YAML merge keys (`<<`) as specified in YAML 1.2.

## Basic Usage

Merge keys allow you to merge the contents of one or more mappings into another mapping:

```yaml
defaults: &defaults
  adapter: postgres
  host: localhost
  port: 5432

development:
  <<: *defaults
  database: dev_db
```

This results in `development` containing all keys from `defaults` plus `database`.

## Features

### Single Merge

Merge a single mapping:

```yaml
base: &base
  key1: value1
  key2: value2

derived:
  <<: *base
  key3: value3
```

### Multiple Merges

Merge multiple mappings using a sequence:

```yaml
base1: &base1
  a: 1

base2: &base2
  b: 2

combined:
  <<: [*base1, *base2]
  c: 3
```

### Override Behavior

Explicit keys in the mapping override merged keys:

```yaml
defaults: &defaults
  port: 5432

custom:
  <<: *defaults
  port: 3306  # Overrides the merged value
```

Keys can be specified before or after the merge key - explicit keys always take precedence.

### Nested Mappings

Important: Merge keys perform shallow merging. Nested mappings are replaced entirely, not deep-merged:

```yaml
base: &base
  settings:
    a: 1
    b: 2

derived:
  <<: *base
  settings:
    c: 3  # Replaces the entire 'settings' mapping
```

Result: `derived.settings` contains only `{c: 3}`.

## Implementation Details

- Merge keys are processed during the compose phase
- The merge value must be a mapping or sequence of mappings
- Invalid merge values result in a construction error
- Merge order: First mapping in sequence has lowest precedence
- Explicit keys always override merged keys
- Fully compliant with YAML 1.2 specification

## Examples

### Environment Configuration

```yaml
common: &common
  app_name: MyApp
  log_level: info
  timeout: 30

development:
  <<: *common
  log_level: debug
  debug: true

production:
  <<: *common
  log_level: error
  optimize: true
```

### Database Configuration

```yaml
db_defaults: &db_defaults
  pool_size: 10
  timeout: 30
  retry: 3

postgres: &postgres
  <<: *db_defaults
  driver: postgresql
  port: 5432

mysql: &mysql
  <<: *db_defaults
  driver: mysql
  port: 3306

app_db:
  <<: *postgres
  host: localhost
  database: myapp
```

## Error Handling

The following will produce errors:

```yaml
# Error: Merge value must be a mapping
invalid1:
  <<: "string value"

# Error: Sequence must contain only mappings
invalid2:
  <<: [*mapping, "string"]
```

## Limitations

- Deep merging is not supported (by design, per YAML spec)
- Merge keys only work in mappings, not in sequences
- Circular references through merge keys may cause issues
