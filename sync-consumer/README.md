# Swarm Sync Consumer

Consumer implementation for Swarm.

**Important Note**:

If you use features like `ENABLE_INITIAL_SYNC`,
ensure you remove them once the consumer finishes and restart the service.

---

## Setup

```yml
  swarm-consumer:
    image: nbittich/sync-consumer
    environment:
      SWARM_BASE_URL: http://job-manager
      SWARM_USERNAME: bnb
      SWARM_PASSWORD: bnb
      DELTA_ENDPOINT: http://search/update
      ENABLE_DELTA_PUSH: "true"
      CHUNK_SIZE: 100
      TARGET_GRAPH: http://mu.semte.ch/graphs/public
      ROOT_OUTPUT_DIR: /consumer-files
      SPARQL_ENDPOINT: http://triplestore:8890/sparql
    volumes:
      - ./data/files/consumer-files/besluiten:/consumer-files/
    restart: always
    logging: *default-logging
    labels:
      - "logging=true"
```

## Configuration

The application can be configured through the following environment variables:

| **Environment Variable** | **Description**                                      | **Default Value**                       | **Required** |
| ------------------------ | ---------------------------------------------------- | --------------------------------------- | ------------ |
| `ENABLE_INITIAL_SYNC`    | Enables initial synchronization.                     | `false`                                 | No           |
| `CRON_EXPRESSION`        | Cron expression to schedule tasks.                   | `0 * * * * * * ` (Every minutes)        | No           |
| `SWARM_BASE_URL`         | Base URL for Swarm API.                              | N/A                                     | Yes          |
| `SWARM_USERNAME`         | Username for Swarm authentication.                   | N/A                                     | Yes          |
| `SWARM_PASSWORD`         | Password for Swarm authentication.                   | N/A                                     | Yes          |
| `DELTA_ENDPOINT`         | Endpoint for pushing delta changes.                  | N/A                                     | No           |
| `ENABLE_DELTA_PUSH`      | Enables delta push functionality.                    | `false`                                 | No           |
| `DELETE_FILES`           | Enables deletion of files post-processing.           | `true`                                  | No           |
| `CHUNK_SIZE`             | Number of items to process in one batch.             | `1024`                                  | No           |
| `TARGET_GRAPH`           | Target graph for SPARQL operations.                  | N/A                                     | Yes          |
| `SWARM_GRAPH`            | Swarm graph to store sync state                      | http://bittich.be/graphs/swarm-consumer | No           |
| `ROOT_OUTPUT_DIR`        | Directory for output files.                          | `/share`                                | No           |
| `HEAP_SIZE_MB`           | Heap size to limit allocation when reading ttl files | 50                                      | No           |
| `DELTA_BUFFER_SLOT_CAP`  | Size of the buffer allocated for accumulating delta. | 32768 (default)                         | No           |
| `DELTA_SLEEP_MS`         | Sleep time between two delta push calls              | 100 (default)                           | No           |

---
