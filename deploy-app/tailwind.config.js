/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{svelte,ts,js}"],
  theme: {
    container: { center: true, padding: "1rem" },
    extend: {
      colors: {
        /* shadcn-style semantic tokens */
        background: "hsl(var(--background) / <alpha-value>)",
        foreground: "hsl(var(--foreground) / <alpha-value>)",
        card: {
          DEFAULT: "hsl(var(--card) / <alpha-value>)",
          foreground: "hsl(var(--card-foreground) / <alpha-value>)",
        },
        popover: {
          DEFAULT: "hsl(var(--popover) / <alpha-value>)",
          foreground: "hsl(var(--popover-foreground) / <alpha-value>)",
        },
        primary: {
          DEFAULT: "hsl(var(--primary) / <alpha-value>)",
          foreground: "hsl(var(--primary-foreground) / <alpha-value>)",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary) / <alpha-value>)",
          foreground: "hsl(var(--secondary-foreground) / <alpha-value>)",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive) / <alpha-value>)",
          foreground: "hsl(var(--destructive-foreground) / <alpha-value>)",
        },
        input: "hsl(var(--input) / <alpha-value>)",
        ring: "hsl(var(--ring) / <alpha-value>)",

        /* Backwards-compatible aliases so existing class names work. */
        bg: "hsl(var(--background) / <alpha-value>)",
        panel: "hsl(var(--card) / <alpha-value>)",
        border: "hsl(var(--border) / <alpha-value>)",
        /* `text-muted` in our components means "dim text". Map to the
           muted-foreground token so existing usage keeps working; use
           `bg-secondary` for the muted *surface*. */
        muted: "hsl(var(--muted-foreground) / <alpha-value>)",
        accent: {
          DEFAULT: "hsl(var(--accent) / <alpha-value>)",
          foreground: "hsl(var(--accent-foreground) / <alpha-value>)",
        },
        ok:   "hsl(var(--success) / <alpha-value>)",
        warn: "hsl(var(--warning) / <alpha-value>)",
        err:  "hsl(var(--destructive) / <alpha-value>)",
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      fontFamily: {
        sans: [
          '"Google Sans Flex"', '"Google Sans"',
          "ui-sans-serif", "system-ui", "-apple-system", "BlinkMacSystemFont",
          '"Segoe UI"', "Inter", "sans-serif",
        ],
        mono: [
          "ui-monospace", "SFMono-Regular", "Menlo", "Monaco", "Consolas",
          '"Liberation Mono"', '"Courier New"', "monospace",
        ],
      },
      fontSize: {
        // Denser, shadcn-ish scale.
        xs:   ["11.5px", { lineHeight: "16px" }],
        sm:   ["13px",   { lineHeight: "18px" }],
        base: ["14px",   { lineHeight: "20px" }],
        lg:   ["16px",   { lineHeight: "24px" }],
        xl:   ["18px",   { lineHeight: "26px" }],
      },
    },
  },
  plugins: [],
};
