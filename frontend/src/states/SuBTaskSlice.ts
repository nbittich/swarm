import { createSlice, createAsyncThunk, PayloadAction } from "@reduxjs/toolkit";
import axios from "axios";
import { CursorPage, SubTask } from "@swarm/models/domain";

interface SubTasksState {
  data: CursorPage<SubTask>;
  loading: boolean;
}

const initialState: SubTasksState = {
  data: { content: [] },
  loading: false,
};

export const fetchSubTasks = createAsyncThunk(
  "subTasks/fetchSubTasks",
  async ({
    jobId,
    taskId,
    next,
    pageSize,
  }: {
    jobId: string;
    taskId: string;
    next: string | null | undefined;
    pageSize: number;
  }) => {
    const params = { limit: pageSize, next };
    const response = await axios.get(
      `/api/jobs/${jobId}/tasks/${taskId}/subtasks`,
      { params },
    );
    return response.data as CursorPage<SubTask>;
  },
);

const subTasksSlice = createSlice({
  name: "subTasks",
  initialState,
  reducers: {
    reset: (state) => {
      state.data = { content: [] };
      state.loading = false;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchSubTasks.pending, (state) => {
        state.loading = true;
      })
      .addCase(
        fetchSubTasks.fulfilled,
        (state, action: PayloadAction<CursorPage<SubTask>>) => {
          state.data = action.payload;
          state.loading = false;
        },
      )
      .addCase(fetchSubTasks.rejected, (state) => {
        state.loading = false;
      });
  },
});

export const { reset } = subTasksSlice.actions;
export default subTasksSlice.reducer;
