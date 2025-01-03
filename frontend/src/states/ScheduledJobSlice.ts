import { createSlice, createAsyncThunk, } from '@reduxjs/toolkit';
import { ScheduledJob, Page, TaskDefinition, statusOptions, Pageable, } from '@swarm/models/domain';
import { message } from 'antd';
import axios from 'axios';

export const addScheduledJob = createAsyncThunk(
    'scheduledJobs/addScheduledJob',
    async (payload: {
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
                throw new Error("Invalid payload");
            }

            const response = await axios.post('/api/scheduled-jobs/new', jobPayload);
            return response.data;
        } catch (error) {
            return rejectWithValue(error);
        }
    }
);

// Async thunk for deleting a scheduled job
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
export const fetchScheduledJobs = createAsyncThunk(
    'scheduledJobs/fetchScheduledJobs',
    async (pagination?: Pageable) => {
        const response = await axios.post('/api/scheduled-jobs', {
            page: (pagination?.page || 1) - 1,
            limit: pagination?.limit || 10,
            filter: pagination?.filter || {}

        });
        return response.data;
    }
);

const scheduledJobsSlice = createSlice({
    name: 'scheduledJobs',
    initialState: {
        refresh: true,
        scheduledJobs: [] as ScheduledJob[],
        loading: false,
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
        refreshScheduledJobs: (state) => {
            state.refresh = true;
        },
        setPagination: (state, action) => {
            state.pagination = { ...state.pagination, ...action.payload };
        },
    },
    extraReducers: (builder) => {
        builder
            .addCase(fetchScheduledJobs.pending, (state) => {
                state.loading = true;
                state.error = null;
                state.refresh = false;;
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
            .addCase(addScheduledJob.pending, (state) => {
                state.loading = true;
            })
            .addCase(addScheduledJob.fulfilled, (state, _) => {
                state.loading = false;
                state.pagination = { pageSize: state.pagination.pageSize, current: 1 };
                state.refresh = true;
                message.success("Scheduled job added");
            })
            .addCase(addScheduledJob.rejected, (state, action) => {
                state.loading = false;
                state.error = action.payload as string;
                message.error("could not add scheduled job!");
            })
            .addCase(deleteScheduledJob.pending, (state) => {
                state.loading = true;
            })
            .addCase(deleteScheduledJob.fulfilled, (state, _) => {
                state.loading = false;
                state.pagination = { pageSize: state.pagination.pageSize, current: 1 };
                state.refresh = true;
                message.success("Scheduled job deleted");
            })
            .addCase(deleteScheduledJob.rejected, (state, action) => {
                state.loading = false;
                state.error = action.payload as string;
                message.error("could not delete scheduled job!");
            });
    },
});

export const { setPagination } = scheduledJobsSlice.actions;

export default scheduledJobsSlice.reducer;

