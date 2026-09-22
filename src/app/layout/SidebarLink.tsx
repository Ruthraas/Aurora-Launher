import { NavLink, useLocation } from "react-router-dom";
import { cn } from "@/lib/utils";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { NavItem } from "./nav-items";

export function SidebarLink({ item, label }: { item: NavItem; label: string }) {
  const Icon = item.icon;
  const { pathname } = useLocation();
  const isActive = item.to === "/" ? pathname === "/" : pathname.startsWith(item.to);

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <NavLink
          to={item.to}
          className={cn(
            "flex h-10 w-10 cursor-pointer items-center justify-center rounded-lg border border-transparent transition-colors duration-150",
            isActive
              ? "border-primary/40 bg-primary/10 text-primary"
              : "text-muted-foreground hover:bg-surface-2 hover:text-foreground",
          )}
        >
          <Icon size={18} strokeWidth={1.75} />
        </NavLink>
      </TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  );
}
