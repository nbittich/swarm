# Swarm

Experimental alternative to the [Harvester](https://github.com/lblod/app-lblod-harvester/)

## Common environment variables

| Variable Name          | Description                                              | Default Value           |
| ---------------------- | -------------------------------------------------------- | ----------------------- |
| `NATS_CONNECTION_URL`  | NATS connection URL for message streaming.               | `nats://localhost:4222` |
| `NATS_ACK_WAIT`        | Wait before redelivering the message again (in seconds). | `600` (10 minutes)      |
| `NATS_MAX_RECONNECT`   | MAX NATS connection retries                              | 5                       |
| `NATS_USERNAME`        | NATS username (optional)                                 | None                    |
| `NATS_PASSWORD`        | NATS password (optional)                                 | None                    |
| `MONGO_HOST`           | Host address of the MongoDB server.                      | `127.0.0.1`             |
| `MONGO_PORT`           | Port number of the MongoDB server.                       | `27017`                 |
| `MONGO_USERNAME`       | Username for MongoDB authentication.                     | `root`                  |
| `MONGO_PASSWORD`       | Password for MongoDB authentication.                     | `root`                  |
| `MONGO_ADMIN_DATABASE` | Admin database name for MongoDB.                         | `admin`                 |
| `MONGO_CONN_TIMEOUT`   | Timeout for MongoDB connections (in seconds).            | None                    |
