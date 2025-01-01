import { combineReducers, } from '@reduxjs/toolkit';
import subtaskReducerSlice from './SuBTaskSlice';
import themeSlice from './ThemeSlice';
const RootReducer = combineReducers({
  subTasks: subtaskReducerSlice,
  theme: themeSlice,
});
export default RootReducer;
