# Job Manager

Service responsible for the creation, management, and scheduling of jobs.
It depends on MongoDB for storage, Meilisearch for indexing, Virtuoso and NATS for messaging. Ensure all required environment variables are configured
in the different readme's.

## Prerequisites

- **MongoDB**: Ensure you have a running MongoDB instance.
- **NATS**: A NATS messaging server is required.
- **Meilisearch**: A Meilisearch server is required.
- **Virtuoso**: A Virtuoso database is required.

## Environment variable

| Variable Name          | Type     | Description                                                                               | Default Value               |
| ---------------------- | -------- | ----------------------------------------------------------------------------------------- | ----------------------------|
| `JWT_SECRET`           | `String` | Holds JWT keys loaded from the `JWT_SECRET` environment variable.                         | None (must be set)          |
| `ROOT_OUTPUT_DIR`      | `String` | Path for the root output directory, defaults to `/share` if `ROOT_OUTPUT_DIR` is not set. | `/share`                    |
| `APPLICATION_NAME`     | `String` | Application name, loaded from the `APPLICATION_NAME` environment variable.                | `job-manager`               |
| `SERVICE_HOST`         | `String` | Service host, loaded from the `SERVICE_HOST` environment variable.                        | `127.0.0.1`                 |
| `SERVICE_PORT`         | `String` | Service port, loaded from the `SERVICE_PORT` environment variable.                        | `80`                        |
| `JOB_DEFINITIONS_PATH` | `String` | Path to the job definitions JSON file                                                     | `/definitions.json`         |
| `BODY_SIZE_LIMIT`      | `String` | Body size limit for downloads (in mb)                                                     | `50`                        |
| `MEILISEARCH_URL`      | `String` | URL of the Meilisearch instance                                                           | `N/A`                       |
| `MEILISEARCH_KEY`      | `String` | API key used to authenticate with Meilisearch                                             | `N/A`                       |
| `INDEX_CONFIG_PATH`    | `String` | Path to the JSON configuration file describing search indexing rules and schema           | `N/A`                       |
| `SCHEDULE_START_DELAY` | `number` | Delay in seconds before starting the job scheduler                                        | `300`                       |
| `MAX_CONCURRENT_JOB`    | `number`| Maximum concurrent scheduled job                                                          | `5`                         |

## Endpoints

### Jobs

- **`GET /jobs`**

  - Description: Retrieves all jobs.
  - Parameters: None.

- **`POST /jobs/new`**
  - Description: Creates a new job based on the provided definition.
  - Parameters:
    - `definitionId` (string): ID of the job definition.
    - `jobName` (string, optional): Name of the job.
    - `targetUrl` (string, optional): Target URL for the job.

### Scheduled Jobs

- **`GET /scheduled-jobs`**

  - Description: Retrieves all scheduled jobs.
  - Parameters: None.

- **`POST /scheduled-jobs/new`**

  - Description: Creates a new scheduled job.
  - Parameters:
    - `definitionId` (string): ID of the job definition.
    - `targetUrl` (string): Target URL for the job.
    - `cronExpr` (string): Cron expression for scheduling.

- **Cron Expression Format**

```
┌───────────── second (0-59)
│ ┌───────────── minute (0-59)
│ │ ┌───────────── hour (0-23)
│ │ │ ┌───────────── day of the month (1-31)
│ │ │ │ ┌───────────── month (1-12 or JAN-DEC)
│ │ │ │ │ ┌───────────── day of the week (0-7 or SUN-SAT)
│ │ │ │ │ │ ┌───────────── year (optional)
│ │ │ │ │ │ │
* * * * * * *

The following rules apply:

   * A field may be an asterisk (*), which always stands for "first-last". For the "day of the month" or "day of the week" fields, a question mark (?) may be used instead of an asterisk.
   * Ranges of numbers are expressed by two numbers separated with a hyphen (-). The specified range is inclusive.
   * Following a range (or *) with /n specifies the interval of the number's value through the range.
   * English names can also be used for the "month" and "day of week" fields. Use the first three letters of the particular day or month (case does not matter).

Example expressions:

    "0 0 * * * *" = the top of every hour of every day.
    "*/10 * * * * *" = every ten seconds.
    "0 0 8-10 * * *" = 8, 9 and 10 o'clock of every day.
    "0 0 6,19 * * *" = 6:00 AM and 7:00 PM every day.
    "0 0/30 8-10 * * *" = 8:00, 8:30, 9:00, 9:30, 10:00 and 10:30 every day.
    "0 0 9-17 * * MON-FRI" = on the hour nine-to-five weekdays
    "0 0 0 25 12 ?" = every Christmas Day at midnight

```

### Job Definitions

- **`GET /job-definitions`**
  - Description: Retrieves all job definitions.
  - Parameters: None.

### Tasks

- **`GET /job/:job_id/tasks`**
  - Description: Retrieves all tasks for a specific job.
  - Parameters:
    - `job_id` (string): ID of the job (path parameter).


### SPARQL Endpoint

- **`POST /sparql`**
  - Description: Sparql endpoint.
  - Parameters: `query`.

### Search Endpoint

- **`POST /:index/search`**
  - Description: Search endpoint.

### Other Routes

- **`ANY /**`\*\*
  - Description: Returns a `404 Not Found` response for unspecified routes.
    |
