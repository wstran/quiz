import { combineReducers, configureStore } from '@reduxjs/toolkit';
import config from './config';

const rootReducer = combineReducers({ config });

export default configureStore({
    reducer: rootReducer,
    middleware: (getDefaultMiddleware) => getDefaultMiddleware({ serializableCheck: false }),
});

export type IRootState = ReturnType<typeof rootReducer>;
