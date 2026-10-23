/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{svelte,js,ts}"],
  theme: {
    extend: {
      colors: {
        "gc-bg": "#0F1013",
        "gc-titlebar": "#0A0B0D",
        "gc-sidebar": "#141518",
        "gc-card": "#1A1B1E",
        "gc-hover": "#232428",
        "gc-border": "#2A2B2F",
        "gc-accent": "#00E7A0",
        "gc-accent-hover": "#00C98A",
        "gc-text": "#FFFFFF",
        "gc-text-sec": "#8B8D91",
        "gc-online": "#00E7A0",
        "gc-offline": "#5A5B5F",
        "gc-error": "#FF4D4D",
      },
    },
  },
  plugins: [],
};
