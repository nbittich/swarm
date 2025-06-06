import { createSlice, createAsyncThunk, PayloadAction } from "@reduxjs/toolkit";
import {
  BatchResponse,
  BatchStatus,
  IndexConfiguration,
  IndexStatistics,
  SearchQueryRequest,
  SearchQueryResponse,
} from "@swarm/models/domain";
import axios from "axios";

interface SearchState {
  configurations: IndexConfiguration[];
  batches: BatchResponse;
  loading: boolean;
  searchResult: SearchQueryResponse | undefined;
  searching: boolean;
  error: string | undefined;
  indexStatistics: IndexStatistics | undefined;
}

const initialState: SearchState = {
  configurations: [],
  batches: { batches: [] },
  loading: false,
  searchResult: undefined,
  searching: false,
  error: undefined,
  indexStatistics: undefined,
};

export const fetchSearchBatches = createAsyncThunk(
  "fetchSearchBatches",
  async ({
    statuses,
    next,
  }: {
    statuses: BatchStatus[] | undefined;
    next: number | undefined;
  }) => {
    const response = await axios.post<BatchResponse>("/api/search/batches", {
      statuses: statuses?.length ? statuses : null,
      next: next && next > 0 ? next : null,
    });
    const sortedByStartedAt = response.data.batches.sort(
      (a, b) =>
        new Date(b.startedAt).getTime() - new Date(a.startedAt).getTime(),
    );
    const batchResponse = {
      ...response.data,
      batches: sortedByStartedAt,
    } as BatchResponse;

    return batchResponse;
  },
);

export const fetchSearchConfigurations = createAsyncThunk(
  "fetchConfigurations",
  async () => {
    const response = await axios.get<IndexConfiguration[]>(
      "/api/search-configuration",
    );
    return response.data;
  },
);

export const fetchIndexStatistics = createAsyncThunk(
  "fetchIndexStatistics",
  async (index: string | undefined) => {
    if (!index) return undefined;
    const response = await axios.get<IndexStatistics>(
      `/api/search/${index}/stats`,
    );
    return response.data;
  },
);

export const performSearch = createAsyncThunk(
  "performSearch",
  async ({
    index,
    request,
  }: {
    index: string;
    request: SearchQueryRequest;
  }) => {
    const response = await axios.post<SearchQueryResponse>(
      `/api/search/${index}`,
      request,
    );
    return response.data;
  },
);

const searchSlice = createSlice({
  name: "search",
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
      .addCase(
        fetchSearchConfigurations.fulfilled,
        (state, action: PayloadAction<IndexConfiguration[]>) => {
          state.configurations = action.payload;
          state.loading = false;
        },
      )
      .addCase(fetchSearchConfigurations.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message || "Error fetching configurations";
      })
      // fetchSearchBatches
      .addCase(fetchSearchBatches.pending, (state) => {
        state.loading = true;
        state.error = undefined;
      })
      .addCase(
        fetchSearchBatches.fulfilled,
        (state, action: PayloadAction<BatchResponse>) => {
          state.batches = action.payload;
          state.loading = false;
        },
      )
      .addCase(fetchSearchBatches.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message || "Error fetching batches";
      })
      // fetchIndexStatistics
      .addCase(fetchIndexStatistics.pending, (state) => {
        state.loading = true;
        state.error = undefined;
      })
      .addCase(
        fetchIndexStatistics.fulfilled,
        (state, action: PayloadAction<IndexStatistics | undefined>) => {
          state.indexStatistics = action.payload;
          state.loading = false;
        },
      )
      .addCase(fetchIndexStatistics.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message || "Error fetching stats";
      })
      // performSearch
      .addCase(performSearch.pending, (state) => {
        state.searching = true;
        state.error = undefined;
      })
      .addCase(
        performSearch.fulfilled,
        (state, action: PayloadAction<SearchQueryResponse>) => {
          state.searchResult = action.payload;
          state.searching = false;
        },
      )
      .addCase(performSearch.rejected, (state, action) => {
        state.searching = false;
        state.error = action.error.message || "Error performing search";
      });
  },
});

export const { clearSearchResult } = searchSlice.actions;

export default searchSlice.reducer;
