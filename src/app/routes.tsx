import type { RouteObject } from "react-router-dom";
import { AppShell } from "./layout/AppShell";
import { HomePage } from "@/features/home/HomePage";
import { InstancesPage } from "@/features/instances/InstancesPage";
import { InstanceDetailPage } from "@/features/instances/InstanceDetailPage";
import { DiscoverPage } from "@/features/discover/DiscoverPage";
import { AccountsPage } from "@/features/accounts/AccountsPage";
import { SettingsPage } from "@/features/settings/SettingsPage";

export const routes: RouteObject[] = [
  {
    element: <AppShell />,
    children: [
      { path: "/", element: <HomePage /> },
      { path: "/instances", element: <InstancesPage /> },
      { path: "/instances/:id", element: <InstanceDetailPage /> },
      { path: "/discover", element: <DiscoverPage /> },
      { path: "/accounts", element: <AccountsPage /> },
      { path: "/settings", element: <SettingsPage /> },
    ],
  },
];
