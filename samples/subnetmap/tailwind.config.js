/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        subnet: {
          blue: "#3b82f6",
          hover: "#2563eb",
          dark: "#0f172a",
          darker: "#020617",
          border: "#334155",
          secondary: "#94a3b8",
          green: "#22c55e",
          yellow: "#eab308",
          red: "#ef4444",
          purple: "#a855f7",
        },
      },
    },
  },
  plugins: [],
};
