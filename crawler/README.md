# Crawler

Website crawler/scraper.

## Application Configuration

This application relies on several environment variables to control its runtime behavior. Below is a list of supported environment variables.

## Environment Variables

| Variable Name                       | Description                                            | Type              | Default Value                                                            |
| ----------------------------------- | ------------------------------------------------------ | ----------------- | ------------------------------------------------------------------------ |
| `MAX_RETRY`                         | The maximum number of retry attempts.                  | Integer (`usize`) | `3`                                                                      |
| `DEFAULT_USER_AGENT`                | User-Agent string for HTTP requests.                   | String            | `Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0` |
| `REQUEST_TIMEOUT_SEC`               | Timeout for HTTP requests (in seconds).                | Integer (`u64`)   | `30`                                                                     |
| `MIN_DELAY_MILLIS`                  | Minimum delay between requests (ms).                   | Integer (`u64`)   | `20`                                                                     |
| `MAX_DELAY_MILLIS`                  | Maximum delay between requests (ms).                   | Integer (`u64`)   | `250`                                                                    |
| `BUFFER_BACK_PRESSURE`              | Buffer size for backpressure.                          | Integer (`usize`) | `16`                                                                     |
| `INTERESTING_PROPERTIES`            | Optional comma separated properties to filter results. | String            | Not set (Optional)                                                       |
| `HTTP_CACHE_PATH`                   | HTTP Cache Path                                        | String            | temp directory                                                           |
| `CONNECTION_POOL_MAX_IDLE_PER_HOST` | Pool max idle per host                                 | Integer (`usize`) | `usize::MAX`                                                             |
| `DEFAULT_ACCEPT`                    | Default accept                                         | String            | `text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8`        |
