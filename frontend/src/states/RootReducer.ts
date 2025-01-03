import { combineReducers, } from '@reduxjs/toolkit';
import subtaskReducerSlice from './SuBTaskSlice';
import scheduledJobsSlice from './ScheduledJobSlice';
import themeSlice from './ThemeSlice';
import { jobDefinitionsReducer } from './JobDefinitionSlice';
const RootReducer = combineReducers({
    subTasks: subtaskReducerSlice,
    scheduledJobs: scheduledJobsSlice,
    jobDefinitions: jobDefinitionsReducer,
    theme: themeSlice,
});
export default RootReducer;
