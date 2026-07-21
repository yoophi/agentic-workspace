import * as React from "react";
import { cn } from "../lib/utils";
export const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(({ className, ...props }, ref) => <input ref={ref} className={cn("h-12 w-full rounded-xl border border-white/10 bg-[#11140f] px-4 text-sm text-white outline-none placeholder:text-[#62675d] focus:border-[#d8ff65]/50 focus:ring-2 focus:ring-[#d8ff65]/10", className)} {...props} />);
Input.displayName = "Input";
