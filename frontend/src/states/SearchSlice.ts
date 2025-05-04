
import { createSlice, createAsyncThunk, PayloadAction } from '@reduxjs/toolkit';
import { IndexConfiguration, IndexStatistics, SearchQueryRequest, SearchQueryResponse } from '@swarm/models/domain';
import axios from 'axios';


interface SearchState {
    configurations: IndexConfiguration[];
    loading: boolean;
    searchResult: SearchQueryResponse | undefined;
    searching: boolean;
    error: string | undefined;
    indexStatistics: IndexStatistics | undefined
}

const initialState: SearchState = {
    configurations: [],
    loading: false,
    searchResult: undefined,
    searching: false,
    error: undefined,
    indexStatistics: undefined
};


export const fetchSearchConfigurations = createAsyncThunk('fetchConfigurations', async () => {
    const response = await axios.get<IndexConfiguration[]>('/api/search-configuration');
    return response.data;
});

export const fetchIndexStatistics = createAsyncThunk('fetchIndexStatistics', async (index: string) => {
    const response = await axios.get<IndexStatistics>(`/api/search/${index}/stats`);
    return response.data;
});

export const performSearch = createAsyncThunk('performSearch', async ({ index, request }: { index: string; request: SearchQueryRequest }) => {
    const response = await axios.post<SearchQueryResponse>(`/api/search/${index}`, request);
    return response.data;
});

const searchSlice = createSlice({
    name: 'search',
    initialState,
    reducers: {
        clearSearchResult(state) {
            state.searchResult = undefined;
        },
    },
    extraReducers: (builder) => {
        builder
            // fetchSearchConfigurations
            .addCase(fetchSearchConfigurations.pending, (state) => {
                state.loading = true;
                state.error = undefined;
            })
            .addCase(fetchSearchConfigurations.fulfilled, (state, action: PayloadAction<IndexConfiguration[]>) => {
                state.configurations = action.payload;
                state.loading = false;
            })
            .addCase(fetchSearchConfigurations.rejected, (state, action) => {
                state.loading = false;
                state.error = action.error.message || 'Error fetching configurations';
            })
            // fetchIndexStatistics
            .addCase(fetchIndexStatistics.pending, (state) => {
                state.loading = true;
                state.error = undefined;
            })
            .addCase(fetchIndexStatistics.fulfilled, (state, action: PayloadAction<IndexStatistics>) => {
                state.indexStatistics = action.payload;
                state.loading = false;
            })
            .addCase(fetchIndexStatistics.rejected, (state, action) => {
                state.loading = false;
                state.error = action.error.message || 'Error fetching stats';
            })
            // performSearch
            .addCase(performSearch.pending, (state) => {
                state.searching = true;
                state.error = undefined;
            })
            .addCase(performSearch.fulfilled, (state, action: PayloadAction<SearchQueryResponse>) => {
                state.searchResult = action.payload;
                state.searching = false;
            })
            .addCase(performSearch.rejected, (state, action) => {
                state.searching = false;
                state.error = action.error.message || 'Error performing search';
            });
    },
});

export const { clearSearchResult } = searchSlice.actions;

export default searchSlice.reducer;
