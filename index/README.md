
## Environment Variables

| Variable Name                | Description                                                                                                        | Required | Example Value                            |
|------------------------------|--------------------------------------------------------------------------------------------------------------------|----------|------------------------------------------|
| `UUID_COMPLEMENT_PREDICATE`  | RDF predicate used to extract UUIDs from triples for indexing.                                                     | Yes      | `http://example.org/uuid`                |
| `MEILISEARCH_URL`            | URL of the Meilisearch instance.                                                                                   | Yes      | `http://localhost:7700`                  |
| `MEILISEARCH_KEY`            | API key used to authenticate with Meilisearch.                                                                     | Yes      | `masterKey`                              |
| `INDEX_CONFIG_PATH`          | Path to the JSON configuration file describing indexing rules and schema.                                          | Yes      | `/config/index_config.json`              |
| `INDEX_MAX_WAIT_FOR_TASK`    | Maximum time in seconds to wait for a meilisearch task. Default to 3600 sec.                                       | No       | `3600`                                   |
| `RESET_INDEX`                | If set to true, forces re-creation of the Meilisearch index.                                                       | No       | `true`                                   |
| `RESET_INDEX_NAME`           | Optional custom name for the Meilisearch index to reset. It must be defined in the index config                    | No       | `custom_index_name`                      |
| `CHUNK_SIZE`                 | Size of each batch tasks                                                                                           | No       | `100`                                    |



### Note
If `RESET_INDEX` is set to `true`, the service **will not consume any events**.
It will perform a reset, then wait doing nothing until restarted with `RESET_INDEX` set to `false`.  
If restarted with `RESET_INDEX` still `true`, it will just reset again.  
There is **no persistence** of the reset state, so be sure to toggle the value when it's done.
