# SparqlClient

A Rust library for interacting with SPARQL endpoints.
It provides functionalities to send SPARQL queries and updates with support for retries and customizable request timeouts.

## Features

- SPARQL query and update support
- Automatic retries on failures
- Configurable via environment variables

## Usage

### Environment Variables

The following environment variables are used by the library:

| Variable Name               | Description                                       | Default Value | Example                     |
| --------------------------- | ------------------------------------------------- | ------------- | --------------------------- |
| `SPARQL_ENDPOINT`           | The SPARQL endpoint URL(required)                 | None          | `http://example.org/sparql` |
| `TARGET_GRAPH`              | The target graph for the SPARQL queries(required) | None          | `http://example.org/graph`  |
| `SPARQL_MAX_RETRY`          | Maximum number of retries for failed queries      | `5`           | `3`                         |
| `REQUEST_TIMEOUT_SEC`       | Timeout for HTTP requests in seconds              | `30`          | `60`                        |
| `SPARQL_RETRY_DELAY_MILLIS` | Delay before next retry                           | `5000`        | `5000`                      |
