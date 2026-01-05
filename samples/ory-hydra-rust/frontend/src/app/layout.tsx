import type { Metadata } from "next";
import "./globals.css";
import Header from "@/components/shared/Header";

export const metadata: Metadata = {
  title: "DONADONA - Engineer Assignment Platform",
  description: "Gamified incident and project management with engineer assignments",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased min-h-screen bg-gray-50">
        <Header />
        <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
          {children}
        </main>
      </body>
    </html>
  );
}
