import { useNavigate, useLocation } from 'react-router-dom';
import { Lock, Cookie, Settings, Bot, LucideIcon } from 'lucide-react';

export const Navbar = () => {
  const navigate = useNavigate();
  const location = useLocation();

  const navItems: Array<{ path: string; label: string; icon: LucideIcon }> = [
    { path: '/login', label: '扫码登录', icon: Lock },
    { path: '/cookies', label: 'Cookies管理', icon: Cookie },
    { path: '/dependency', label: '依赖检测', icon: Settings },
    { path: '/playwright', label: 'Playwright服务', icon: Bot },
  ];

  const isActive = (path: string) => location.pathname === path;

  return (
    <nav className="bg-white shadow-sm border-b border-gray-200">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between items-center h-16">
          <div className="flex items-center">
            <h1 className="text-lg sm:text-xl font-bold text-gray-900">微博登录助手</h1>
          </div>

          <div className="flex space-x-1">
            {navItems.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  key={item.path}
                  onClick={() => navigate(item.path)}
                  className={`
                    flex items-center px-2 sm:px-4 py-2 rounded-lg text-xs sm:text-sm font-medium transition-colors
                    ${isActive(item.path)
                      ? 'bg-blue-100 text-blue-700'
                      : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900'
                    }
                  `}
                >
                  <Icon className="w-4 h-4 sm:w-5 sm:h-5 mr-1 sm:mr-2" />
                  <span className="hidden sm:inline">{item.label}</span>
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </nav>
  );
};
