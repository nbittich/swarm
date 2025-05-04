export type JSONValue =
    | string
    | number
    | boolean
    | JSONObject
    | JSONValue[];

export interface JSONObject {
    [x: string]: JSONValue;
}

export interface Pageable {
    page: number,
    limit: number,
    filter?: object,
    sort?: object
}

export interface Page<T> {
    totalElements: number,
    currentPage: number,
    nextPage: number,
    pageSize: number,
    content: T[]
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
    | { type: "fromPreviousStep"; value: { taskId: string; payload?: TaskResult } };

export type TaskResult =
    | { type: "scrapeWebsite"; value: { successCount: number; failureCount: number; manifestFilePath: string } }
    | { type: "extractRDFa"; value: { successCount: number; failureCount: number; manifestFilePath: string } }
    | { type: "filterSHACL"; value: { successCount: number; failureCount: number; manifestFilePath: string } }
    | { type: "complementWithUuid"; value: { successCount: number; failureCount: number; manifestFilePath: string } }
    | { type: "diff"; value: { successCount: number; failureCount: number; manifestFilePath: string } }
    | { type: "publish"; value: { removedTripleFilePath: string; intersectTripleFilePath: string; insertedTripleFilePath: string; failedQueryFilePath: string, diffManifestFilePath: string } }
    | { type: "json"; value: object };

export type SubTaskResult =
    | { type: "scrapeUrl"; value: ScrapeResult }
    | { type: "nTriple"; value: NTripleResult }
    | { type: "diff"; value: DiffResult }
    | { type: "json"; value: object };

export interface ScrapeResult {
    baseUrl: string;
    path: string;
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
    path: string;
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
    { type: "failed", value: [] }
]
export interface UserClaims {
    sub: string;
    firstName?: string;
    lastName?: string;
    email: string;
    exp: number;
}

export function colorForStatus(status: Status): string {
    let color = "default";
    if (status.type === "pending") color = "gray";
    if (status.type === "scheduled") color = "blue";
    if (status.type === "busy") color = "orange";
    if (status.type === "success") color = "green";
    if (status.type === "archived") color = "purple";
    if (status.type === "failed") color = "red";
    return color;
}

export const truncate = (str: string, length: number) => {
    if (str.length > length) {
        return str.substring(0, length) + '...';
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
}
