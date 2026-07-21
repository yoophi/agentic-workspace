import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../lib/utils";

const buttonVariants = cva("inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-xl text-sm font-semibold transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#d8ff65]/50 disabled:pointer-events-none disabled:opacity-40", { variants: { variant: { default: "bg-[#d8ff65] text-[#11150d] shadow-[0_8px_30px_rgba(216,255,101,.18)] hover:bg-[#e4ff91] hover:-translate-y-0.5", secondary: "border border-white/10 bg-white/[.055] text-white hover:bg-white/10", ghost: "text-[#9aa191] hover:bg-white/[.06] hover:text-white" }, size: { default: "h-11 px-5", sm: "h-9 px-3", icon: "h-10 w-10" } }, defaultVariants: { variant: "default", size: "default" } });
export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {}
export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(({ className, variant, size, ...props }, ref) => <button ref={ref} className={cn(buttonVariants({ variant, size, className }))} {...props} />);
Button.displayName = "Button";
