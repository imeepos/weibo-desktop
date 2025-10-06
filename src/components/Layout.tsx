import { ReactNode } from 'react';
import { Navbar } from './Navbar';

interface LayoutProps {
  children: ReactNode;
  showNavbar?: boolean;
}

export const Layout = ({ children, showNavbar = true }: LayoutProps) => {
  return (
    <div className="min-h-screen bg-gray-50">
      {showNavbar && <Navbar />}
      <main>{children}</main>
    </div>
  );
};
