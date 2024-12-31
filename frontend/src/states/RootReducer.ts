import { combineReducers, } from '@reduxjs/toolkit';
import subtaskReducerSlice from './SuBTaskSlice';

const RootReducer = combineReducers({
  subTasks: subtaskReducerSlice
});
export default RootReducer;
