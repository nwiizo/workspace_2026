/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        chirp: {
          blue: "#1d9bf0",
          hover: "#1a8cd8",
          dark: "#15202b",
          darker: "#192734",
          border: "#38444d",
          secondary: "#8899a6",
        },
      },
    },
  },
  plugins: [],
};
