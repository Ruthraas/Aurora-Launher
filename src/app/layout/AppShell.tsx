import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";
import { LoginScreen } from "@/features/auth/LoginScreen";
import { useAccounts } from "@/features/accounts/use-accounts";

export function AppShell() {
  const { data, isLoading } = useAccounts();
  const isLoggedIn = Boolean(data?.activeAccountId);

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        {isLoggedIn ? (
          <>
            <Sidebar />
            <main className="scroll-thin min-w-0 flex-1 overflow-y-auto pb-14 sm:pb-0">
              <Outlet />
            </main>
          </>
        ) : (
          !isLoading && (
            <main className="min-w-0 flex-1">
              <LoginScreen />
            </main>
          )
        )}
      </div>
    </div>
  );
}
