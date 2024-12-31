import { configureStore } from "@reduxjs/toolkit";
import RootReducer from "./RootReducer";
import logger from 'redux-logger';

export const store = configureStore({
  reducer: RootReducer,
  middleware: (getDefaultMiddleware) => getDefaultMiddleware().concat(logger),
})
export type AppDispatch = typeof store.dispatch;
export type RootState = ReturnType<typeof store.getState>;
