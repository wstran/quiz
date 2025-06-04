import { createSlice, PayloadAction } from '@reduxjs/toolkit';

export interface Config {
    [key: string]: any;
};

const initialState = {
    config: null,
} as Config;

const configSlice = createSlice({
    name: 'config',
    initialState: initialState,
    reducers: {
        setConfig(state, action: PayloadAction<((user: Config | null) => Config) | Config | null>) {
            state.config = typeof action.payload === 'function' ? action.payload(state.config) : action.payload;
        },
    },
});

export const {
    setConfig,
} = configSlice.actions;

export default configSlice.reducer;