
## Environment Variables

| Variable Name                | Description                                                                | Required | Example Value                            |
|------------------------------|----------------------------------------------------------------------------|----------|------------------------------------------|
| `UUID_COMPLEMENT_PREDICATE`  | RDF predicate used to extract UUIDs from triples for indexing.             | Yes      | `http://example.org/uuid`                |
| `MEILISEARCH_URL`            | URL of the Meilisearch instance.                                           | Yes      | `http://localhost:7700`                  |
| `MEILISEARCH_KEY`            | API key used to authenticate with Meilisearch.                             | Yes      | `masterKey`                              |
| `INDEX_CONFIG_PATH`          | Path to the JSON configuration file describing indexing rules and schema.  | Yes      | `/config/index_config.json`              |

