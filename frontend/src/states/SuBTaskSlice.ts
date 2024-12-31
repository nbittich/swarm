import { createSlice, createAsyncThunk, PayloadAction } from "@reduxjs/toolkit";
import axios from "axios";
import { SubTask } from "@swarm/models/domain";

interface SubTasksState {
  data: SubTask[];
  loading: boolean;
  lastElementId: string | null;
}

const initialState: SubTasksState = {
  data: [] as SubTask[],
  loading: false,
  lastElementId: null,
};

export const fetchSubTasks = createAsyncThunk(
  "subTasks/fetchSubTasks",
  async ({ jobId, taskId, lastElementId, pageSize }: { jobId: string; taskId: string; lastElementId: string | null; pageSize: number }) => {
    const params = { limit: pageSize, lastElementId };
    const response = await axios.get(`/api/jobs/${jobId}/tasks/${taskId}/subtasks`, { params });
    return response.data as SubTask[];
  }
);

const subTasksSlice = createSlice({
  name: "subTasks",
  initialState,
  reducers: {
    reset: (state) => {
      state.lastElementId = null;
      state.data = [];
      state.loading = false;
    }

  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchSubTasks.pending, (state) => {
        state.loading = true;
      })
      .addCase(fetchSubTasks.fulfilled, (state, action: PayloadAction<SubTask[]>) => {
        const newSubTasks = action.payload;
        state.data = state.lastElementId ? [...state.data, ...newSubTasks as SubTask[]] : newSubTasks;
        state.lastElementId = newSubTasks.length > 0 ? newSubTasks[newSubTasks.length - 1]._id : null;
        state.loading = false;
      })
      .addCase(fetchSubTasks.rejected, (state) => {
        state.loading = false;
      });
  },
});

export const { reset } = subTasksSlice.actions
export default subTasksSlice.reducer;
