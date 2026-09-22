import { Compass, Home, Settings, UserCircle, Boxes, type LucideIcon } from "lucide-react";

export interface NavItem {
  to: string;
  icon: LucideIcon;
  labelKey: string;
}

export const navItems: NavItem[] = [
  { to: "/", icon: Home, labelKey: "nav.home" },
  { to: "/instances", icon: Boxes, labelKey: "nav.instances" },
  { to: "/discover", icon: Compass, labelKey: "nav.discover" },
  { to: "/accounts", icon: UserCircle, labelKey: "nav.accounts" },
];

export const settingsNavItem: NavItem = { to: "/settings", icon: Settings, labelKey: "nav.settings" };
