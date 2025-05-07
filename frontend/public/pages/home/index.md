# Swarm

## Introduction

Swarm is an open-source project that centralizes and makes searchable data published 
from every municipality in Flanders, it scrapes and turns html pages into semantic data that can be queried.

All data is extracted from the official websites of Flemish municipalities, where it's published as RDFa annotated html. 
Swarm scrapes that data, processes it, and republishes it in a usable format.

A sparql endpoint and an index endpoint are publicly available to let you query or search agenda items and sessions more quickly.

This is a hobby project I built in my spare time to play around with new technologies and show how open data from the Flemish government can be used to 
build your own thing, it was largely inspired by [LokaalBeslist](https://lokaalbeslist.vlaanderen.be/).

It is far from perfect and probably has a bunch of issues (I wrote both the RDFa parser and the Turtle parser from scratch),
so if you need this data for a real world use case, I strongly recommend using [LokaalBeslist](https://lokaalbeslist.vlaanderen.be/) instead, which is much more battle-tested.

On the other hand, it is fully open-source and I (try) to maintain and improve it whenever I can. If you see anything that could be improved,
feel free to let me know! If you want to contribute, you are more than welcome.

## How It Works

Swarm works with configurable & schedulable jobs (called job definitions), each made up of a series of tasks, 
and each task is a microservice that has one goal. 

They communicate with events through a [nats](https://nats.io/) broker. When a task is set to success, the next task starts and so on.

The main pipeline looks like this — and it's easy to extend by adding more steps and updating the job definition:

- **Collect**  – Starts with a website url and scrapes all relevant html pages
- **Extract**  – Parses RDFa annotations from the html and converts them into N-Triples
- **Filter**   – Validates and cleans the data using SHACL shapes to apply quality rules
- **Add uuid** – Adds technical uuids to RDF subjects for better traceability and management
- **Diff**     – Compares with the previous job’s output to figure out what was added, deleted, or unchanged
- **Publish**  – Pushes new triples into a Virtuoso triplestore and removes outdated ones
- **Index**    – Updates the Meilisearch index with the latest data
- **Archive**  – Marks the previous job as archived, for cleanup

By working with events, we keep the pipeline clean and easy to trace. 

Jobs can be scheduled using cron expressions, and the whole setup is generic enough to adapt to other domains.

Swarm also has a dedicated microservice (called sync-consumer) that lets third parties plug into the pipeline; when a job finishes successfully, 
the system generates an archive containing the new triples to insert, the triples to delete, and the intersection with the previous job (for initial sync).

This makes it easy to keep your own triplestore in sync, even if you're starting from scratch.

Swarm is fully open source and built to be extended! 

For example, I was thinking of an extra step that would convert the triples into a dataset for [A.I fine-tuning](https://en.wikipedia.org/wiki/Fine-tuning_(deep_learning))

If you'd like to have a look, try it locally or contribute:

- [Fork the project on GitHub](https://github.com/nbittich/swarm)
- [Run it via Docker](https://github.com/nbittich/app-swarm)
- [Deploy it with Ansible](https://github.com/nbittich/ansible-deployment)
