"use client";

import Link from "next/link";
import { useState, useEffect } from "react";

interface User {
  email: string;
  role: string;
}

export default function Header() {
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    const cookieRow = document.cookie
      .split("; ")
      .find((row) => row.startsWith("user_info="));

    if (cookieRow) {
      try {
        const userInfoCookie = cookieRow.substring("user_info=".length);
        const decodedCookie = decodeURIComponent(userInfoCookie);
        const userInfo = JSON.parse(atob(decodedCookie));
        setUser(userInfo);
      } catch {
        // Invalid user_info cookie
      }
    }
  }, []);

  const isManager = user?.role === "manager" || user?.role === "platform_admin";
  const isPlatformAdmin = user?.role === "platform_admin";

  return (
    <header className="bg-gradient-to-r from-indigo-600 to-purple-600 shadow-lg">
      <nav className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between h-16">
          <div className="flex items-center">
            <Link href="/" className="flex items-center space-x-2">
              <span className="text-2xl font-bold text-white tracking-tight">
                DONADONA
              </span>
              <span className="text-xs text-indigo-200 hidden sm:block">
                Engineer Assignment Platform
              </span>
            </Link>

            {/* Navigation Links */}
            {user && (
              <div className="hidden md:ml-10 md:flex md:space-x-1">
                <Link
                  href="/dashboard"
                  className="text-indigo-100 hover:text-white hover:bg-indigo-500/50 px-3 py-2 rounded-md text-sm font-medium transition"
                >
                  Dashboard
                </Link>
                <Link
                  href="/incidents"
                  className="text-indigo-100 hover:text-white hover:bg-indigo-500/50 px-3 py-2 rounded-md text-sm font-medium transition"
                >
                  Incidents
                </Link>
                <Link
                  href="/projects"
                  className="text-indigo-100 hover:text-white hover:bg-indigo-500/50 px-3 py-2 rounded-md text-sm font-medium transition"
                >
                  Projects
                </Link>
                {isManager && (
                  <>
                    <Link
                      href="/engineers"
                      className="text-indigo-100 hover:text-white hover:bg-indigo-500/50 px-3 py-2 rounded-md text-sm font-medium transition"
                    >
                      Engineers
                    </Link>
                    <Link
                      href="/recruitment"
                      className="text-indigo-100 hover:text-white hover:bg-indigo-500/50 px-3 py-2 rounded-md text-sm font-medium transition"
                    >
                      Recruitment
                    </Link>
                  </>
                )}
                <Link
                  href="/leaderboard"
                  className="text-indigo-100 hover:text-white hover:bg-indigo-500/50 px-3 py-2 rounded-md text-sm font-medium transition"
                >
                  Leaderboard
                </Link>
              </div>
            )}
          </div>

          <div className="flex items-center">
            {user ? (
              <div className="relative">
                <button
                  onClick={() => setIsMenuOpen(!isMenuOpen)}
                  className="flex items-center text-sm font-medium text-white hover:text-indigo-100 transition"
                >
                  <div className="w-8 h-8 rounded-full bg-white/20 flex items-center justify-center mr-2">
                    <span className="text-sm font-bold">
                      {user.email.charAt(0).toUpperCase()}
                    </span>
                  </div>
                  <span className="hidden sm:block">{user.email}</span>
                  <span className="ml-2 px-2 py-0.5 text-xs bg-white/20 rounded-full">
                    {user.role.replace("_", " ")}
                  </span>
                </button>

                {isMenuOpen && (
                  <div className="absolute right-0 mt-2 w-48 rounded-lg shadow-lg bg-white ring-1 ring-black ring-opacity-5 z-50">
                    <div className="py-1">
                      {isPlatformAdmin && (
                        <Link
                          href="/tenants"
                          className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100"
                          onClick={() => setIsMenuOpen(false)}
                        >
                          Manage Tenants
                        </Link>
                      )}
                      <a
                        href="/api/auth/logout"
                        className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100"
                      >
                        Sign Out
                      </a>
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <a
                href="/api/auth/login"
                className="inline-flex items-center px-4 py-2 border border-white/30 text-sm font-medium rounded-md text-white bg-white/10 hover:bg-white/20 transition"
              >
                Sign In
              </a>
            )}
          </div>
        </div>
      </nav>
    </header>
  );
}
