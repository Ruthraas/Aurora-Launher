import { useState } from "react";
import { UserCircle, Fingerprint, CalendarDays, Users, Shirt, LogOut } from "lucide-react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "@/components/page-header";
import { EmptyState } from "@/components/empty-state";
import { SectionLabel } from "@/components/section-label";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useAccounts } from "./use-accounts";
import { useLogout, useRemoveAccount, useSetActiveAccount } from "./use-account-mutations";
import { AddOfflineForm } from "./AddOfflineForm";
import { SkinManager } from "./SkinManager";
import { SkinBodyRender } from "./SkinBodyRender";
import { useAccountTexture } from "./use-account-texture";
import type { Account } from "@/lib/tauri/commands/accounts";

const dateFormatter = new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "short", year: "numeric" });

function ActiveProfileCard({ account }: { account: Account }) {
  const { data: skinDataUrl } = useAccountTexture(account.id, "skin", Boolean(account.skinFile));

  return (
    <div className="flex h-full flex-col rounded-xl border border-border bg-card p-6">
      <div className="flex flex-col gap-8 lg:flex-row">
        <div className="flex shrink-0 flex-col items-center gap-3 self-center lg:w-48 lg:self-start lg:border-r lg:border-border lg:pr-8">
          {skinDataUrl ? (
            <SkinBodyRender skinDataUrl={skinDataUrl} height={240} />
          ) : (
            <div className="flex h-[240px] w-[120px] items-center justify-center rounded-lg border border-dashed border-border text-3xl font-semibold text-muted-foreground">
              {account.username.charAt(0).toUpperCase()}
            </div>
          )}
          <div className="flex items-center gap-2">
            <p className="text-base font-bold text-foreground">{account.username}</p>
            <span className="rounded-full border border-primary/40 bg-primary/10 px-2 py-0.5 text-[0.7rem] text-primary">
              Ativa
            </span>
          </div>
        </div>

        <div className="min-w-0 flex-1">
          <SkinManager account={account} />
        </div>
      </div>
    </div>
  );
}

function DetailRow({ icon: Icon, label, value }: { icon: typeof Fingerprint; label: string; value: string }) {
  return (
    <div className="flex items-start gap-3 border-b border-border py-3 last:border-0">
      <Icon size={15} className="mt-0.5 shrink-0 text-muted-foreground" />
      <div className="min-w-0">
        <p className="text-xs text-muted-foreground">{label}</p>
        <p className="truncate text-sm font-medium text-foreground">{value}</p>
      </div>
    </div>
  );
}

function AccountDetailsCard({ account, totalAccounts }: { account: Account; totalAccounts: number }) {
  return (
    <div className="flex h-full flex-col rounded-xl border border-border bg-card p-6">
      <SectionLabel>Detalhes da conta</SectionLabel>
      <div className="mt-2 flex flex-1 flex-col">
        <DetailRow icon={Fingerprint} label="UUID offline" value={account.uuid} />
        <DetailRow icon={CalendarDays} label="Criada em" value={dateFormatter.format(new Date(account.addedAt))} />
        <DetailRow icon={Users} label="Contas nesse launcher" value={String(totalAccounts)} />
        <DetailRow
          icon={Shirt}
          label="Skin"
          value={account.skinFile ? (account.capeFile ? "Personalizada · com capa" : "Personalizada") : "Padrão (Steve/Alex)"}
        />
      </div>
    </div>
  );
}

export function AccountsPage() {
  const { t } = useTranslation();
  const { data, isLoading } = useAccounts();
  const removeAccount = useRemoveAccount();
  const setActiveAccount = useSetActiveAccount();
  const logout = useLogout();
  const pending = removeAccount.isPending || setActiveAccount.isPending;
  const [confirmingRemoveId, setConfirmingRemoveId] = useState<string | null>(null);
  const confirmingRemoveAccount = data?.accounts.find((a) => a.id === confirmingRemoveId);

  const activeAccount = data?.accounts.find((a) => a.id === data.activeAccountId);
  const otherAccounts = data?.accounts.filter((a) => a.id !== data.activeAccountId) ?? [];

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title={t("nav.accounts")}
        actions={
          activeAccount && (
            <Button variant="outline" size="sm" className="gap-1.5" disabled={logout.isPending} onClick={() => logout.mutate()}>
              <LogOut size={13} />
              Sair
            </Button>
          )
        }
      />

      <div className="scroll-thin flex-1 overflow-y-auto p-6">
        <div className="flex flex-col gap-6">
          {isLoading ? (
            <Skeleton className="h-64 w-full rounded-xl" />
          ) : !data || data.accounts.length === 0 ? (
            <EmptyState icon={UserCircle} title={t("accounts.emptyTitle")} description={t("accounts.emptyDescription")} />
          ) : (
            activeAccount && (
              <div className="grid gap-6 lg:grid-cols-3">
                <div className="lg:col-span-2">
                  <ActiveProfileCard account={activeAccount} />
                </div>
                <AccountDetailsCard account={activeAccount} totalAccounts={data.accounts.length} />
              </div>
            )
          )}

          <div className="grid gap-6 lg:grid-cols-2">
            {otherAccounts.length > 0 && (
              <div className="flex flex-col gap-3">
                <SectionLabel>Outras contas</SectionLabel>
                <div className="flex flex-col gap-2">
                  {otherAccounts.map((account) => (
                    <div
                      key={account.id}
                      className="flex items-center justify-between rounded-lg border border-border bg-card px-4 py-2.5"
                    >
                      <div className="flex items-center gap-2.5">
                        <span className="flex h-7 w-7 items-center justify-center rounded-full bg-surface-2 text-xs font-semibold text-muted-foreground">
                          {account.username.charAt(0).toUpperCase()}
                        </span>
                        <p className="text-sm font-medium text-foreground">{account.username}</p>
                      </div>
                      <div className="flex gap-1.5">
                        <Button variant="outline" size="sm" disabled={pending} onClick={() => setActiveAccount.mutate(account.id)}>
                          Usar
                        </Button>
                        <Button variant="ghost" size="sm" disabled={pending} onClick={() => setConfirmingRemoveId(account.id)}>
                          Remover
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            <div className={otherAccounts.length > 0 ? "flex flex-col gap-3" : "flex flex-col gap-3 lg:col-span-2"}>
              <SectionLabel>Adicionar conta</SectionLabel>
              <AddOfflineForm />
            </div>
          </div>
        </div>
      </div>

      <ConfirmDialog
        open={Boolean(confirmingRemoveAccount)}
        onOpenChange={(next) => !next && setConfirmingRemoveId(null)}
        title={`Remover "${confirmingRemoveAccount?.username}"?`}
        description="A conta some da lista — pra usar de novo é só recriar com o mesmo nome (offline não guarda senha)."
        confirmLabel="Remover"
        onConfirm={() => {
          if (confirmingRemoveAccount) removeAccount.mutate(confirmingRemoveAccount.id);
          setConfirmingRemoveId(null);
        }}
      />
    </div>
  );
}
