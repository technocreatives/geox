# geox

Shim to simplify using PostGIS types with sqlx and async-graphql.

## Feature flags:

- **async-graphql**: enable GraphQL types
- **sqlx05**: enable conversions for sqlx 0.5
- **serde1**: enable serde serialisation and deserialisation

## Running tests locally

1. `docker run -d -e POSTGRES_PASSWORD=password -p 5432:5432 --name geox ghcr.io/baosystems/postgis:latest`
2. `cargo test`
3. ???
4. PROFIT!
