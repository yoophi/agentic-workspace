import * as React from "react";
import { cn } from "../lib/utils";
export const Select = React.forwardRef<HTMLSelectElement, React.SelectHTMLAttributes<HTMLSelectElement>>(({ className, children, ...props }, ref) => <select ref={ref} className={cn("h-10 w-full appearance-none rounded-lg border border-white/10 bg-[#11140f] px-3 text-sm text-[#d9ddcf] outline-none focus:border-[#d8ff65]/50", className)} {...props}>{children}</select>);
Select.displayName = "Select";
