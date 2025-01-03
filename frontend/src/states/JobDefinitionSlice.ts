import { createSlice, createAsyncThunk } from '@reduxjs/toolkit';
import axios from 'axios';
import { JobDefinition } from '@swarm/models/domain';

export const fetchJobDefinitions = createAsyncThunk(
    'jobDefinitions/fetchJobDefinitions',
    async () => {
        const response = await axios.get('/api/job-definitions');
        return response.data;
    }
);

const jobDefinitionsSlice = createSlice({
    name: 'jobDefinitions',
    initialState: {
        jobDefinitions: [] as JobDefinition[],
        loading: false,
        error: null as string | null,
    },
    reducers: {},
    extraReducers: (builder) => {
        builder
            .addCase(fetchJobDefinitions.pending, (state) => {
                state.loading = true;
                state.error = null;
            })
            .addCase(fetchJobDefinitions.fulfilled, (state, action) => {
                state.loading = false;
                state.jobDefinitions = action.payload;
            })
            .addCase(fetchJobDefinitions.rejected, (state, action) => {
                state.loading = false;
                state.error = action.error.message || 'Failed to fetch job definitions';
            });
    },
});

export const jobDefinitionsReducer = jobDefinitionsSlice.reducer;
