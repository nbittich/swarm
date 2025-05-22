# Swarm

Swarm is an open-source project designed to centralize and make searchable the data published by municipalities in Flanders. 

It takes advantage of the Flemish government's [open data initiative](https://www.vlaanderen.be/agentschap-binnenlands-bestuur/blikvangers/lokale-besluiten-als-gelinkte-open-data), which mandates local administrations to publish their decisions as linked open data since 2019.

All data is extracted from the official websites of Flemish municipalities, where it is published as [RDFa](https://fr.wikipedia.org/wiki/RDFa) annotated html. Swarm scrapes this data, processes it, and republishes it in a usable format.

Largely inspired by a similar existing project, it's **not intended** to replace any existing tools; I use it primarily as a sandbox for trying out new ideas in data extraction and processing. For production-level work, I'd strongly recommend using a more reliable solutions.

It is currently running on a [KS-2](https://eco.ovhcloud.com/fr/kimsufi/ks-2/) server with ~5TB disk drives, 32 GB of RAM, and an Intel Xeon-D 1540 (8 core, 16 threads) CPU. At the time of writing, 2 million agenda items have been extracted.

### How It Works

Swarm works with configurable & schedulable jobs (called job definitions), each made up of a series of tasks, and each task is a microservice with a specific responsibility, communicating through events via a [NATS](https://nats.io/) message broker. This event-driven approach keeps the pipeline organized and easy to follow, where each task's completion triggers the next.

The primary data processing pipeline consists of the following steps:

1. **collect**: Scrapes relevant html pages from specified website urls
2. **extract**: Parses RDFa annotations from the collected html and converts them into N-Triples
3. **filter**: Applies shacl shapes to validate and clean the extracted data
4. **add-uuid**: Assigns unique identifiers to RDF subjects for traceability and management
5. **diff**: Compares the current job's output with the previous one to identify additions, deletions, and unchanged data
6. **publish**: Inserts new triples and removes outdated ones in the triplestore
7. **index**: Synchronizes the Meilisearch index with the latest data
8. **archive**: Marks the previous job as archived and ready for deletion

![](/pages/home/diagram.png)

This pipeline is designed to be extensible, and new steps can be easily added by updating the job definitions. Jobs can be scheduled using cron expressions, and the entire system is adaptable to other domains beyond Flemish municipal data.

Swarm also has a dedicated microservice called `sync-consumer`, that lets third parties consume the extracted data without having to run their own Swarm instance; when a job finishes successfully, the system generates an archive containing the new triples to insert, the triples to delete, and the intersection with the previous job (for initial sync), making it straightforward to keep external triplestores in sync. 
If you'd like to try it, please [contact me](https://bittich.be/contact) so I can send you the credentials and instructions.

Swarm is primarily developed in Rust, but other languages can be used to extend it. For example, the filter step is written in Java, as the Jena shacl library is more robust.


Swarm uses Virtuoso as its triplestore and Meilisearch for indexing, because it's fast and offers a great developer experience. Job and task metadata are stored in MongoDB, since that data doesn’t really benefit from being stored as semantic data. This setup keeps the pipeline simpler and faster, but I might revisit this choice if a good reason to use linked data for the metadata comes up in the future.

### Custom Components

Swarm is a handmade project that relies on custom components:

- [an RDFa parser](https://github.com/nbittich/graph-rdfa-processor)
- [a turtle/N-Triple parser](https://github.com/nbittich/tortank)
- [a web crawler/scraper](https://github.com/nbittich/swarm/tree/master/crawler)

These components are not intended for production use due to their experimental nature.

### Getting Involved

Swarm is completely open-source. Whether you're interested in enhancing existing functionality, adding new features, or simply exploring the codebase, I welcome your contributions!

#### To get started:

- [Fork the project on GitHub](https://github.com/nbittich/swarm)
- [Run it locally with Docker](https://github.com/nbittich/app-swarm)
- [Deploy it with Ansible](https://github.com/nbittich/ansible-deployment)

If you have suggestions for improvement or encounter any issues, please don't hesitate to open an issue or submit a pull request.

### Future Directions

- Extending the pipeline to add more processing steps, like converting the extracted triples into datasets that are suitable for [fine-tuning AI models](https://en.wikipedia.org/wiki/Fine-tuning_(deep_learning)). 
- Write a parser for [Microdata](https://html.spec.whatwg.org/multipage/microdata.html)

### Request for Removal

If you identify personal or sensitive data sourced from your publicly available website and would like it removed in accordance with the General Data Protection Regulation (GDPR), please [contact me](https://bittich.be/contact). I will review your request and take appropriate action without undue delay.
