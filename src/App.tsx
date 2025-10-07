import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ErrorBoundary } from './components/ErrorBoundary';
import { Layout } from './components/Layout';
import { DependencyCheckPage } from './pages/DependencyCheckPage';
import { LoginPage } from './pages/LoginPage';
import { CookiesListPage } from './pages/CookiesListPage';
import { PlaywrightServicePage } from './pages/PlaywrightServicePage';
import { RedisConfigPage } from './pages/RedisConfigPage';

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
            <Route path="/playwright" element={<PlaywrightServicePage />} />
            <Route path="/redis" element={<RedisConfigPage />} />
          </Routes>
        </Layout>
      </BrowserRouter>
    </ErrorBoundary>
  );
}

export default App;
