import { createSlice, createAsyncThunk, } from '@reduxjs/toolkit';
import { ScheduledJob, JobSchedulerStatus, Page, TaskDefinition, statusOptions, Pageable, } from '@swarm/models/domain';
import { message } from 'antd';
import axios from 'axios';

const DEFAULT_PAGEABLE = {
    page: 1,
    limit: 10,
    filter: {},
    sort: {
        creationDate: -1
    }
} as Pageable
export const upsertScheduledJob = createAsyncThunk(
    'scheduledJobs/upsertScheduledJob',
    async (payload: {
        __id?: string;
        definitionId: string;
        jobName?: string;
        cronExpr: string;
        taskDefinition: TaskDefinition | null;
        targetUrl?: string;
        status?: string;
    }, { rejectWithValue }) => {
        try {
            const jobPayload = {
                definitionId: payload.definitionId,
                name: payload.jobName,
                cronExpr: payload.cronExpr,
                taskDefinition: payload.taskDefinition
            };

            if (payload.taskDefinition?.payload.type === "scrapeUrl" && payload.targetUrl) {
                jobPayload.taskDefinition = {
                    ...payload.taskDefinition,
                    payload: {
                        type: "scrapeUrl",
                        value: payload.targetUrl
                    },
                };
            } else if (payload.taskDefinition?.payload.type === "cleanup" && payload.status && statusOptions.some(s => s.type === payload.status)) {
                jobPayload.taskDefinition = {
                    ...payload.taskDefinition,
                    payload: {
                        type: "cleanup",
                        value: statusOptions.find(s => s.type === payload.status)!
                    },
                };
            } else {
                throw new Error("Invalid payload " + JSON.stringify(payload));
            }
            let response;
            if (payload.__id) {
                response = await axios.post(`/api/scheduled-jobs/${payload.__id}/update`, jobPayload);
            } else {
                response = await axios.post('/api/scheduled-jobs/new', jobPayload);
            }

            return response.data;
        } catch (error) {
            return rejectWithValue(error);
        }
    }
);

export const deleteScheduledJob = createAsyncThunk(
    'scheduledJobs/deleteScheduledJob',
    async (jobId: string, { rejectWithValue }) => {
        try {
            await axios.delete(`/api/scheduled-jobs/${jobId}`);
            return jobId;
        } catch (error) {
            return rejectWithValue(error);
        }
    }
);
export const runScheduledJobManually = createAsyncThunk(
    'scheduledJobs/runScheduledJobManually',
    async (jobId: string, { rejectWithValue }) => {
        try {
            await axios.post(`/api/scheduled-jobs/${jobId}/run`);
            return jobId;
        } catch (error) {
            return rejectWithValue(error);
        }
    }
);
export const fetchScheduledJobs = createAsyncThunk(
    'scheduledJobs/fetchScheduledJobs',
    async (pagination?: Pageable) => {
        const response = await axios.post('/api/scheduled-jobs', {
            page: (pagination?.page || 1) - 1,
            limit: pagination?.limit || 10,
            filter: pagination?.filter || {},
            sort: pagination?.sort || {},

        });
        return response.data;
    }
);
export const fetchScheduledJobStatus = createAsyncThunk(
    'scheduledJobs/fetchScheduledJobStatus',
    async () => {
        const response = await axios.post('/api/scheduled-jobs/status', {});
        return response.data;
    }
);
export const toggleScheduledJobStatus = createAsyncThunk(
    'scheduledJobs/toggleScheduledJobStatus',
    async (status: JobSchedulerStatus) => {
        await axios.post(`/api/scheduled-jobs/${status.status === 'running' ? 'pause' : 'start'}`, {});
        return status;
    }
);
const scheduledJobsSlice = createSlice({
    name: 'scheduledJobs',
    initialState: {
        scheduledJobStatus: undefined as JobSchedulerStatus | undefined,
        scheduledJobs: [] as ScheduledJob[],
        loading: false,
        pageable: DEFAULT_PAGEABLE,
        pagination: {
            current: 1,
            pageSize: 10,
            total: undefined,
        } as {
            current: number,
            pageSize: number,
            total?: number
        },
        error: null as string | null,
    },
    reducers: {
        setPagination: (state, action) => {
            state.pagination = { ...state.pagination, ...action.payload };
        },
        setPageable: (state, action) => {
            state.pageable = { ...state.pageable, ...action.payload };
        }
    },
    extraReducers: (builder) => {
        builder
            .addCase(toggleScheduledJobStatus.pending, (state) => {
                state.loading = true;
                state.error = null;
            })
            .addCase(toggleScheduledJobStatus.fulfilled, (state, action) => {
                state.loading = false;
                state.scheduledJobStatus = action.payload;

            })
            .addCase(toggleScheduledJobStatus.rejected, (state, action) => {
                state.loading = false;
                state.error = action.error.message || 'Failed to to toggle scheduled job status';
            })
            .addCase(fetchScheduledJobStatus.pending, (state) => {
                state.loading = true;
                state.error = null;
            })
            .addCase(fetchScheduledJobStatus.fulfilled, (state, action) => {
                state.loading = false;
                state.scheduledJobStatus = action.payload;

            })
            .addCase(fetchScheduledJobStatus.rejected, (state, action) => {
                state.loading = false;
                state.error = action.error.message || 'Failed to fetch scheduled jobs status';
            })
            .addCase(fetchScheduledJobs.pending, (state) => {
                state.loading = true;
                state.error = null;
            })
            .addCase(fetchScheduledJobs.fulfilled, (state, action) => {
                state.loading = false;
                const page: Page<ScheduledJob> = action.payload;
                state.scheduledJobs = page.content;
                state.pagination = {
                    ...state.pagination,
                    total: page.totalElements,
                    current: page.currentPage + 1,
                };
            })
            .addCase(fetchScheduledJobs.rejected, (state, action) => {
                state.loading = false;
                state.error = action.error.message || 'Failed to fetch scheduled jobs';
            })
            .addCase(upsertScheduledJob.pending, (state) => {
                state.loading = true;
            })
            .addCase(upsertScheduledJob.fulfilled, (state, _) => {
                state.loading = false;
                state.pagination = { pageSize: state.pagination.pageSize, current: 1 };
                state.pageable = { ...DEFAULT_PAGEABLE }; // trick; do not change unless you know what you do
                message.success("Scheduled job added or modified");
            })
            .addCase(upsertScheduledJob.rejected, (state, action) => {
                state.loading = false;
                state.error = action.payload as string;
                message.error("could not upsert scheduled job!");
            })
            .addCase(runScheduledJobManually.pending, (state) => {
                state.loading = true;
            })
            .addCase(runScheduledJobManually.fulfilled, (state, _) => {
                state.loading = false;
                message.success("Scheduled job started.");
            })
            .addCase(runScheduledJobManually.rejected, (state, action) => {
                state.loading = false;
                state.error = action.payload as string;
                message.error("could not start scheduled job!");
            })
            .addCase(deleteScheduledJob.pending, (state) => {
                state.loading = true;
            })
            .addCase(deleteScheduledJob.fulfilled, (state, _) => {
                state.loading = false;
                state.pagination = { pageSize: state.pagination.pageSize, current: 1 };
                state.pageable = { ...DEFAULT_PAGEABLE }; // trick; do not change unless you know what you do
                message.success("Scheduled job deleted");
            })
            .addCase(deleteScheduledJob.rejected, (state, action) => {
                state.loading = false;
                state.error = action.payload as string;
                message.error("could not delete scheduled job!");
            });
    },
});

export const { setPagination, setPageable } = scheduledJobsSlice.actions;

export default scheduledJobsSlice.reducer;

