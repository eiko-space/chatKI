import type { ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost";
}

export function Button({ variant = "secondary", className, ...props }: ButtonProps) {
  return (
    <button
      className={cn(
        "inline-flex h-9 items-center justify-center gap-2 rounded-lg px-4 text-sm font-medium",
        "transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-40",
        variant === "primary" && "bg-accent text-bg hover:bg-accent/85",
        variant === "secondary" && "border border-line bg-surface-2 text-text hover:bg-surface-3",
        variant === "ghost" && "text-muted hover:bg-surface-2 hover:text-text",
        className,
      )}
      {...props}
    />
  );
}
