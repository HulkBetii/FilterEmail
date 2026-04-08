/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "SF Pro Display",
          "SF Pro Text",
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "sans-serif",
        ],
      },
      colors: {
        shell: "#eef3f8",
        ink: "#0f172a",
        mist: "#475569",
        glass: "rgba(255, 255, 255, 0.84)",
        primary: "#0f6cbd",
        success: "#15803d",
        warn: "#b45309",
        danger: "#dc2626",
        panel: "#ffffff",
        line: "rgba(148, 163, 184, 0.24)",
      },
      boxShadow: {
        float: "0 24px 60px rgba(15, 23, 42, 0.16)",
        glass: "0 14px 36px rgba(15, 23, 42, 0.1)",
      },
      backgroundImage: {
        aura:
          "radial-gradient(circle at top left, rgba(56, 189, 248, 0.34), transparent 32%), radial-gradient(circle at top right, rgba(255, 255, 255, 0.98), transparent 30%), linear-gradient(180deg, #f8fbff 0%, #e9f0f7 100%)",
      },
    },
  },
  plugins: [],
};
