import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ErrorBoundary } from './components/ErrorBoundary';
import { Layout } from './components/Layout';
import { DependencyCheckPage } from './pages/DependencyCheckPage';
import { LoginPage } from './pages/LoginPage';
import { CookiesListPage } from './pages/CookiesListPage';

function App() {
  return (
    <ErrorBoundary>
      <BrowserRouter>
        <Layout>
          <Routes>
            <Route path="/" element={<Navigate to="/login" replace />} />
            <Route path="/login" element={<LoginPage />} />
            <Route path="/cookies" element={<CookiesListPage />} />
            <Route path="/dependency" element={<DependencyCheckPage />} />
          </Routes>
        </Layout>
      </BrowserRouter>
    </ErrorBoundary>
  );
}

export default App;
