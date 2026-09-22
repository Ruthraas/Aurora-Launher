import { useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useAddOfflineAccount } from "./use-account-mutations";

export function AddOfflineForm() {
  const [nickname, setNickname] = useState("");
  const mutation = useAddOfflineAccount();

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    mutation.mutate(nickname, { onSuccess: () => setNickname("") });
  }

  return (
    <form onSubmit={handleSubmit} className="rounded-lg border border-border bg-card p-4">
      <p className="text-sm font-medium text-foreground">Conta offline</p>
      <p className="mt-1 text-xs text-muted-foreground">
        Só precisa de um nome de usuário. Funciona em modo um jogador e em servidores com{" "}
        <span className="font-medium text-foreground">online-mode=false</span>. Servidores que exigem
        conta original não aceitam essa conta, e skins não aparecem.
      </p>

      <label className="mt-3 block text-xs text-muted-foreground" htmlFor="offline-nickname">
        Nome de usuário
      </label>
      <Input
        id="offline-nickname"
        value={nickname}
        onChange={(event) => setNickname(event.target.value)}
        placeholder="[seu nick]"
        maxLength={16}
        className="mt-1"
      />

      {mutation.isError && <p className="mt-2 text-xs text-destructive">{mutation.error.message}</p>}

      <Button
        type="submit"
        variant="outline"
        size="sm"
        className="mt-3"
        disabled={mutation.isPending || nickname.trim().length === 0}
      >
        {mutation.isPending ? "Adicionando…" : "Adicionar conta offline"}
      </Button>
    </form>
  );
}
