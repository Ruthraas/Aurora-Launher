import { useState, type FormEvent } from "react";
import { AuroraLogo } from "@/components/aurora-logo";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useAccounts } from "@/features/accounts/use-accounts";
import { useAddOfflineAccount, useSetActiveAccount } from "@/features/accounts/use-account-mutations";

/** Portão de entrada do launcher: sem conta ativa, é a única coisa que
 * aparece — nada de navegação/sidebar até escolher ou criar uma conta
 * offline. Contas já existentes (ex.: usuário removeu a ativa) podem
 * ser reusadas direto daqui, sem precisar passar pela tela Contas. */
export function LoginScreen() {
  const { data, isLoading } = useAccounts();
  const [nickname, setNickname] = useState("");
  const addAccount = useAddOfflineAccount();
  const setActive = useSetActiveAccount();

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    addAccount.mutate(nickname, { onSuccess: () => setNickname("") });
  }

  const accounts = data?.accounts ?? [];

  return (
    <div className="flex h-full flex-col items-center justify-center gap-8 p-6">
      <div className="flex flex-col items-center gap-3">
        <AuroraLogo size={48} />
        <div className="text-center">
          <h1 className="text-xl font-bold text-foreground">Aurora Launcher</h1>
          <p className="text-sm text-muted-foreground">Entra com uma conta offline pra começar</p>
        </div>
      </div>

      <div className="flex w-full max-w-sm flex-col gap-5">
        {!isLoading && accounts.length > 0 && (
          <div className="flex flex-col gap-2">
            {accounts.map((account) => (
              <button
                key={account.id}
                type="button"
                disabled={setActive.isPending}
                onClick={() => setActive.mutate(account.id)}
                className="flex cursor-pointer items-center gap-3 rounded-lg border border-border bg-card px-4 py-2.5 text-left transition-colors hover:border-primary/40 hover:bg-surface-2 disabled:pointer-events-none disabled:opacity-50"
              >
                <span className="flex h-8 w-8 items-center justify-center rounded-full bg-primary/15 text-sm font-semibold text-primary">
                  {account.username.charAt(0).toUpperCase()}
                </span>
                <span className="text-sm font-medium text-foreground">{account.username}</span>
              </button>
            ))}
            {setActive.isError && <p className="text-xs text-destructive">{setActive.error.message}</p>}
          </div>
        )}

        <form onSubmit={handleSubmit} className="flex flex-col gap-2.5 rounded-lg border border-border bg-card p-4">
          <p className="text-sm font-medium text-foreground">
            {accounts.length > 0 ? "Ou criar outra conta offline" : "Criar conta offline"}
          </p>
          <Input
            value={nickname}
            onChange={(event) => setNickname(event.target.value)}
            placeholder="Nome de usuário"
            maxLength={16}
            autoFocus
          />
          {addAccount.isError && <p className="text-xs text-destructive">{addAccount.error.message}</p>}
          <Button type="submit" disabled={addAccount.isPending || nickname.trim().length === 0}>
            {addAccount.isPending ? "Entrando…" : "Entrar"}
          </Button>
        </form>

        <p className="text-center text-xs text-muted-foreground">
          Só precisa de um nome — funciona em modo um jogador e em servidores com online-mode=false. Servidores
          que exigem conta original não aceitam essa conta.
        </p>
      </div>
    </div>
  );
}
