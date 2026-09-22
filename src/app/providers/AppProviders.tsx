import type { ReactNode } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { I18nextProvider } from "react-i18next";
import { BrowserRouter } from "react-router-dom";
import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import { queryClient } from "./query-client";
import { ErrorBoundary } from "./ErrorBoundary";

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <ErrorBoundary>
      <I18nextProvider i18n={i18n}>
        <QueryClientProvider client={queryClient}>
          <TooltipProvider delayDuration={200}>
            <BrowserRouter>{children}</BrowserRouter>
          </TooltipProvider>
        </QueryClientProvider>
      </I18nextProvider>
    </ErrorBoundary>
  );
}
