# Swarm Sync consumer

Consumer implementation for Swarm.
Important note: to keep things as simple as possible, this service doesn't store any state; you only have the logs.
If you decide to use features like `START_FROM_DELTA_TIMESTAMP` or `ENABLE_INITIAL_SYNC`,
please make sure to remove them once the consumer finished, and restart the service.
