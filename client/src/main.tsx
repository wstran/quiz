import React, { Suspense } from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter as Router, Route, Routes } from 'react-router-dom';
import { Provider } from 'react-redux';
import { Buffer } from 'buffer';
import store from '@/storage';
import Index from '@/routes';
import '@/tailwind.css';

if (typeof globalThis.Buffer === 'undefined') {
    globalThis.Buffer = Buffer;
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
        <Suspense>
            <Provider store={store}>
                <Router>
                    <Routes>
                        <Route
                            path='*'
                            element={<Index />}
                        />
                        <Route
                            path='/'
                            element={<Index />}
                        />
                    </Routes>
                </Router>
            </Provider>
        </Suspense>
    </React.StrictMode>
);
