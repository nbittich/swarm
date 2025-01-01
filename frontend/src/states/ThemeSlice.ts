import { createSlice } from '@reduxjs/toolkit';

const themeSlice = createSlice({
  name: 'theme',
  initialState: {
    darkMode: false,
  },
  reducers: {
    toggleTheme(state, action) {
      state.darkMode = action.payload; // action.payload is the boolean value to toggle the theme
    },
  },
});

export const { toggleTheme } = themeSlice.actions;

export default themeSlice.reducer;
