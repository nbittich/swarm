export interface Pageable {
    page: number;
    limit: number;
    filter?: object;
    sort?: object;
}

export interface Page<T> {
    totalElements: number;
    currentPage: number;
    nextPage: number;
    pageSize: number;
    content: T[];
}

export interface JobDefinition {
    id: string;
    name: string;
    allowConcurrentRun: boolean;
    tasks: TaskDefinition[];
}

export interface TaskDefinition {
    name: string;
    order: number;
    payload: Payload;
}

export interface Job {
    _id: string;
    name: string;
    targetUrl?: string;
    rootDir: string;
    creationDate: string;
    modifiedDate?: string;
    status: Status;
    definition: JobDefinition;
}

export interface ScheduledJob {
    _id: string;
    name?: string;
    creationDate: string;
    nextExecution?: string;
    taskDefinition: TaskDefinition;
    definitionId: string;
    cronExpr: string;
}

export interface Task {
    _id: string;
    order: number;
    jobId: string;
    name: string;
    creationDate: string;
    modifiedDate?: string;
    payload: Payload;
    result?: TaskResult;
    hasSubTask: boolean;
    status: Status;
    outputDir: string;
}

export interface CursorPage<T> {
    next?: string;
    current?: string;
    content: T[];
}
export interface SubTask {
    _id: string;
    taskId: string;
    creationDate: string;
    modifiedDate?: string;
    status: Status;
    result?: SubTaskResult;
}

export type Payload =
    | { type: "none" }
    | { type: "archive" }
    | { type: "cleanup"; value: Status }
    | { type: "scrapeUrl"; value: string }
    | {
        type: "fromPreviousStep";
        value: { taskId: string; payload?: TaskResult };
    };

export type TaskResult =
    | {
        type: "scrapeWebsite";
        value: {
            successCount: number;
            failureCount: number;
            manifestFilePath: string;
        };
    }
    | {
        type: "extractRDFa";
        value: {
            successCount: number;
            failureCount: number;
            manifestFilePath: string;
        };
    }
    | {
        type: "filterSHACL";
        value: {
            successCount: number;
            failureCount: number;
            manifestFilePath: string;
        };
    }
    | {
        type: "complementWithUuid";
        value: {
            successCount: number;
            failureCount: number;
            manifestFilePath: string;
        };
    }
    | {
        type: "diff";
        value: {
            successCount: number;
            failureCount: number;
            manifestFilePath: string;
        };
    }
    | {
        type: "publish";
        value: {
            removedTripleFilePath: string;
            intersectTripleFilePath: string;
            insertedTripleFilePath: string;
            failedQueryFilePath: string;
            diffManifestFilePath: string;
        };
    }
    | { type: "json"; value: object };

export type SubTaskResult =
    | { type: "scrapeUrl"; value: ScrapeResult }
    | { type: "nTriple"; value: NTripleResult }
    | { type: "diff"; value: DiffResult }
    | { type: "json"; value: object };

export interface ScrapeResult {
    baseUrl: string;
    path?: string;
    creationDate: string;
}

export interface DiffResult {
    baseUrl: string;
    newInsertPath?: string;
    intersectPath?: string;
    toRemovePath?: string;
    creationDate: string;
}

export interface NTripleResult {
    baseUrl: string;
    len: number;
    path?: string;
    creationDate: string;
}

export type Status =
    | { type: "pending" }
    | { type: "scheduled" }
    | { type: "busy" }
    | { type: "success" }
    | { type: "archived" }
    | { type: "failed"; value: string[] };

export const statusOptions: Status[] = [
    { type: "pending" },
    { type: "scheduled" },
    { type: "busy" },
    { type: "success" },
    { type: "archived" },
    { type: "failed", value: [] },
];
export interface UserClaims {
    sub: string;
    firstName?: string;
    lastName?: string;
    email: string;
    exp: number;
}

export function colorForStatus(status: Status): string {
    if (status.type === "pending") return "gray";
    if (status.type === "scheduled") return "blue";
    if (status.type === "busy") return "orange";
    if (status.type === "success") return "green";
    if (status.type === "archived") return "purple";
    if (status.type === "failed") return "red";
    return "default";
}
export function colorForBatchStatus(status: SearchBatchStatus): string {
    if (status.canceled) return "gray";
    if (status.enqueued) return "blue";
    if (status.processing) return "orange";
    if (status.succeeded) return "green";
    if (status.failed) return "red";
    return "default";
}
export function labelForBatchStatus(
    status: SearchBatchStatus,
): BatchStatus | undefined {
    if (status.canceled) return "canceled";
    if (status.enqueued) return "enqueued";
    if (status.processing) return "processing";
    if (status.succeeded) return "succeeded";
    if (status.failed) return "failed";
    return undefined;
}
export const truncate = (str: string, length: number) => {
    if (str.length > length) {
        return str.substring(0, length) + "...";
    }
    return str;
};

export interface IndexConfiguration {
    name: string;
    rdfType: string[];
    // onPath: string; // fixme, we don't use it
    properties: RdfProperty[];
}

export interface RdfProperty {
    name: string;
    paths: string[];
    optional: boolean;
    config?: RdfPropertyConfig;
}

export interface RdfPropertyConfig {
    visible: boolean;
    jsType?: "date" | "string" | "number" | "url";
}

export interface SearchQueryRequest {
    query?: SearchQueryType;
    neg: boolean;
    sortBy?: string;
    sortDirection?: Order;
    filters?: string;
    offset?: number;
    limit: number;
}

export interface IndexStatistics {
    numberOfDocuments: number;
}

export type SearchQueryType =
    | { type: "word"; value: string }
    | { type: "phrase"; value: string };

export type Order = "asc" | "desc";

export interface SearchQueryResponse {
    hits: Array<Record<string, unknown>>;
    totalHits?: number;
    totalPages?: number;
    page?: number;
    limit?: number;
}

export interface Batch {
    uid: number;
    details?: Details | unknown;
    stats: Stats;
    duration: string;
    startedAt: string;
    finishedAt?: string;
    progress?: Progress;
}

export interface Details {
    receivedDocuments: number;
    indexedDocuments: number;
}

export interface Stats {
    totalNbTasks: number;
    status: SearchBatchStatus;
    types: unknown;
    indexUids: Record<string, number>;
    progressTrace: unknown;
    writeChannelCongestion: unknown;
    internalDatabaseSizes: unknown;
}

export interface Progress {
    steps: Step[];
    percentage: number;
}

export interface Step {
    currentStep: string;
    finished: number;
    total: number;
}

export interface SearchBatchStatus {
    enqueued?: number;
    processing?: number;
    succeeded?: number;
    failed?: number;
    canceled?: number;
}
export interface BatchResponse {
    batches: Batch[];
    next?: number;
    prev?: number;
    current?: number;
}

export type BatchStatus =
    | "enqueued"
    | "processing"
    | "succeeded"
    | "failed"
    | "canceled";

export function getPayloadFromScheduledJob(
    sj: ScheduledJob,
): string | Status | undefined {
    if (sj.taskDefinition.payload.type === "scrapeUrl") {
        return sj.taskDefinition.payload.value;
    } else if (sj.taskDefinition.payload.type === "cleanup") {
        return sj.taskDefinition.payload.value;
    }
    return undefined;
}

export interface JobSchedulerStatus {
    status: "paused" | "running";
}
