import { combineReducers, UnknownAction, } from '@reduxjs/toolkit';
import subtaskReducerSlice from './SuBTaskSlice';
import scheduledJobsSlice from './ScheduledJobSlice';
import jobsSlice from './JobSlice';
import themeSlice from './ThemeSlice';
import { jobDefinitionsReducer } from './JobDefinitionSlice';

const combinedReducer = combineReducers({
    subTasks: subtaskReducerSlice,
    scheduledJobs: scheduledJobsSlice,
    jobs: jobsSlice,
    jobDefinitions: jobDefinitionsReducer,
});

type combinedState = ReturnType<typeof combinedReducer>;

const appReducer = (state: combinedState | undefined, action: UnknownAction): combinedState => {
    if (action.type === '/jobs') {
        state = {
            ...state,
            jobs: {}
        } as unknown as combinedState;
    } else if (action.type === '/scheduled-jobs') {
        state = {
            ...state,
            scheduledJobs: {}
        } as unknown as combinedState;
    }
    return combinedReducer(state, action);
};

const RootReducer = combineReducers({ appReducer, theme: themeSlice });

export default RootReducer;
