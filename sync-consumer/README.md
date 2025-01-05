# Swarm Sync Consumer

Consumer implementation for Swarm.

**Important Note**:

To keep things simple, this service doesn't store any state; you only have the logs.
This will probably change in the future.
If you use features like `START_FROM_DELTA_TIMESTAMP` or `ENABLE_INITIAL_SYNC`,
ensure you remove them once the consumer finishes and restart the service.

---

## Configuration

The application can be configured through the following environment variables:

| **Environment Variable**     | **Description**                            | **Default Value**             | **Required** |
| ---------------------------- | ------------------------------------------ | ----------------------------- | ------------ |
| `ENABLE_INITIAL_SYNC`        | Enables initial synchronization.           | `false`                       | No           |
| `CRON_EXPRESSION`            | Cron expression to schedule tasks.         | `0 * * * * * * ` (Every hour) | No           |
| `SWARM_BASE_URL`             | Base URL for Swarm API.                    | N/A                           | Yes          |
| `SWARM_USERNAME`             | Username for Swarm authentication.         | N/A                           | Yes          |
| `SWARM_PASSWORD`             | Password for Swarm authentication.         | N/A                           | Yes          |
| `START_FROM_DELTA_TIMESTAMP` | Timestamp to start consuming delta from.   | N/A                           | No           |
| `DELTA_ENDPOINT`             | Endpoint for pushing delta changes.        | N/A                           | No           |
| `ENABLE_DELTA_PUSH`          | Enables delta push functionality.          | `false`                       | No           |
| `DELETE_FILES`               | Enables deletion of files post-processing. | `true`                        | No           |
| `CHUNK_SIZE`                 | Number of items to process in one batch.   | `1024`                        | No           |
| `TARGET_GRAPH`               | Target graph for SPARQL operations.        | N/A                           | Yes          |
| `ROOT_OUTPUT_DIR`            | Directory for output files.                | `/share`                      | No           |

---
