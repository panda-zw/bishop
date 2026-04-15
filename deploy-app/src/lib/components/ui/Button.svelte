<script lang="ts" module>
  export type Variant = "default" | "secondary" | "outline" | "ghost" | "destructive" | "link";
  export type Size = "sm" | "md" | "icon";
</script>

<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  interface Props extends HTMLButtonAttributes {
    variant?: Variant;
    size?: Size;
    children: Snippet;
    class?: string;
  }
  let { variant = "default", size = "md", children, class: klass = "", ...rest }: Props = $props();

  const base =
    "inline-flex items-center justify-center gap-2 rounded-md font-medium " +
    "transition-colors focus-visible:outline-none focus-visible:ring-2 " +
    "focus-visible:ring-ring/50 focus-visible:ring-offset-2 focus-visible:ring-offset-background " +
    "disabled:pointer-events-none disabled:opacity-50 " +
    "whitespace-nowrap select-none";

  const variants: Record<Variant, string> = {
    // Strong primary, subtle shadow so it reads as "raised."
    default:
      "bg-primary text-primary-foreground shadow-sm hover:bg-primary/90 active:bg-primary/95",

    // Filled secondary — clearly a button, lower visual weight than primary.
    secondary:
      "bg-secondary text-secondary-foreground hover:bg-secondary/80 active:bg-secondary/90",

    // Outline now carries a soft card bg so it reads as a pressable surface,
    // not a label. Border + slightly elevated fill on hover.
    outline:
      "bg-card text-foreground border border-border shadow-sm " +
      "hover:bg-secondary hover:border-border/80 active:bg-secondary/90",

    // Ghost stays minimal but gets a clear hover + press state.
    ghost:
      "text-foreground/90 hover:bg-secondary hover:text-foreground active:bg-secondary/80",

    destructive:
      "bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90 active:bg-destructive/95",

    link:
      "text-accent underline-offset-4 hover:underline",
  };

  const sizes: Record<Size, string> = {
    sm:   "h-8 px-3 text-xs",
    md:   "h-9 px-4 text-sm",
    icon: "h-8 w-8 p-0 text-sm",
  };
</script>

<button class="{base} {variants[variant]} {sizes[size]} {klass}" {...rest}>
  {@render children()}
</button>
