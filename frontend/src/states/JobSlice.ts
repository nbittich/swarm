import { createSlice, createAsyncThunk } from '@reduxjs/toolkit';
import axios from 'axios';
import { Job, Page, Pageable, statusOptions, TaskDefinition } from '@swarm/models/domain'; // Assuming Job type is defined here
import { message } from 'antd';

const DEFAULT_PAGEABLE = {
    page: 1,
    limit: 10,
    filter: {},
    sort: {
        creationDate: -1
    }
} as Pageable;

export const fetchJobs = createAsyncThunk(
    'jobs/fetchJobs',
    async (pagination?: Pageable) => {
        const response = await axios.post('/api/jobs', {
            page: (pagination?.page || 1) - 1,
            limit: pagination?.limit || 10,
            filter: pagination?.filter || {},
            sort: pagination?.sort || { creationDate: -1 },

        });
        return response.data;
    }
);

export const addJob = createAsyncThunk(
    'jobs/addJob',
    async (values: {
        definitionId: string,
        taskDefinition: TaskDefinition | null,
        jobName?: string,
        targetUrl?: string,
        status?: string,
    }, { rejectWithValue }) => {
        try {
            const payload = {
                definitionId: values.definitionId,
                jobName: values.jobName?.length ? values.jobName : undefined,
                taskDefinition: values?.taskDefinition,
            };
            if (payload.taskDefinition && payload.taskDefinition.payload.type === "scrapeUrl" && values.targetUrl) {
                payload.taskDefinition = {
                    ...payload.taskDefinition,
                    payload: {
                        type: "scrapeUrl",
                        value: values.targetUrl
                    },

                };
            } else if (payload.taskDefinition && payload.taskDefinition.payload.type === "cleanup" && values.status && statusOptions.some(s => s.type === values.status)) {
                payload.taskDefinition = {
                    ...payload.taskDefinition,
                    payload: {
                        type: "cleanup",
                        value: statusOptions.find(s => s.type === values.status)!
                    },

                };
            } else {
                throw Error("invalid payload");
            }
            const response = await axios.post('/api/jobs/new', payload);
            return response.data;
        } catch (error) {
            return rejectWithValue(error);
        }
    }
);

export const deleteJob = createAsyncThunk(
    'jobs/deleteJob',
    async (jobId: string, { rejectWithValue }) => {
        try {
            await axios.delete(`/api/jobs/${jobId}`);
            return jobId;
        } catch (error) {
            return rejectWithValue(error);
        }
    }
);

interface JobState {
    jobs: Job[];
    pageable: Pageable;
    loading: boolean,
    error: string | null;
    pagination: {
        current: number,
        pageSize: number,
        total?: number
    };
}

const initialState: JobState = {
    jobs: [],
    loading: false,
    pageable: DEFAULT_PAGEABLE,
    error: null,
    pagination: {
        current: 1,
        pageSize: 10,
        total: undefined,
    }

};

const jobsSlice = createSlice({
    name: 'jobs',
    initialState,
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
            .addCase(fetchJobs.pending, (state) => {
                state.loading = true;
                state.error = null;
            })
            .addCase(fetchJobs.fulfilled, (state, action) => {
                const page: Page<Job> = action.payload;
                state.jobs = page.content;
                state.pagination = {
                    ...state.pagination,
                    total: page.totalElements,
                    current: page.currentPage + 1,
                };
                state.loading = false;
            })
            .addCase(fetchJobs.rejected, (state, action) => {
                state.loading = false;
                state.error = action.error.message || 'Failed to fetch scheduled jobs';
            })
            .addCase(addJob.pending, (state,) => {
                state.loading = true;
            })
            .addCase(addJob.fulfilled, (state, _) => {
                state.loading = false;
                state.pageable = { ...DEFAULT_PAGEABLE }; // trick; do not change unless you know what you do
                state.pagination = { pageSize: state.pagination.pageSize, current: 1 };
                message.success("Job added");

            })
            .addCase(addJob.rejected, (state, action) => {
                state.loading = false;
                state.error = action.payload as string;
                message.error("could not add job!");
            })
            .addCase(deleteJob.pending, (state,) => {
                state.loading = true;
            })
            .addCase(deleteJob.rejected, (state, action) => {
                state.loading = false;
                state.error = action.payload as string;
                message.error("could not delete job!");
            })
            .addCase(deleteJob.fulfilled, (state, action) => {
                state.loading = false;
                state.jobs = state.jobs.filter(job => job._id !== action.payload);
                state.pagination = { pageSize: state.pagination.pageSize, current: 1 };
                state.pageable = { ...DEFAULT_PAGEABLE }; // trick; do not change unless you know what you do
            });
    },
});

export const { setPagination, setPageable } = jobsSlice.actions;
export default jobsSlice.reducer;
