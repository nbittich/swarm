import { combineReducers, } from '@reduxjs/toolkit';
import subtaskReducerSlice from './SuBTaskSlice';
import scheduledJobsSlice from './ScheduledJobSlice';
import jobsSlice from './JobSlice';
import themeSlice from './ThemeSlice';
import { jobDefinitionsReducer } from './JobDefinitionSlice';
const RootReducer = combineReducers({
    subTasks: subtaskReducerSlice,
    scheduledJobs: scheduledJobsSlice,
    jobs: jobsSlice,
    jobDefinitions: jobDefinitionsReducer,
    theme: themeSlice,
});
export default RootReducer;
