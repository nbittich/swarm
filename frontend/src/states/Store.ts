import { configureStore } from "@reduxjs/toolkit";
import RootReducer from "./RootReducer";
import logger from 'redux-logger';

export const store = configureStore({
  reducer: RootReducer,
  middleware: (getDefaultMiddleware) => import.meta.env.DEV ? getDefaultMiddleware().concat(logger) : getDefaultMiddleware(),
})
export type AppDispatch = typeof store.dispatch;
export type RootState = ReturnType<typeof store.getState>;
