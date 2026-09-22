import { useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Upload, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Account } from "@/lib/tauri/commands/accounts";
import {
  clearAccountCape,
  clearAccountSkin,
  importAccountSkinByUsername,
  uploadAccountCape,
  uploadAccountSkin,
} from "@/lib/tauri/commands/accounts";
import { accountsQueryKey } from "./use-accounts";

/** Só valida que é uma textura de skin/capa plausível (64 de largura,
 * altura 64/32 pra skin, 64/32 pra capa) — não tenta validar mais que
 * isso, o servidor confia no que o usuário mandou. */
function readPngDimensions(file: File): Promise<{ width: number; height: number }> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve({ width: img.width, height: img.height });
    img.onerror = () => reject(new Error("arquivo não é uma imagem válida"));
    img.src = URL.createObjectURL(file);
  });
}

async function fileToBytes(file: File): Promise<number[]> {
  const buffer = await file.arrayBuffer();
  return Array.from(new Uint8Array(buffer));
}

export function SkinManager({ account }: { account: Account }) {
  const queryClient = useQueryClient();
  const [username, setUsername] = useState("");
  const skinInputRef = useRef<HTMLInputElement>(null);
  const capeInputRef = useRef<HTMLInputElement>(null);

  function invalidate() {
    void queryClient.invalidateQueries({ queryKey: accountsQueryKey });
    void queryClient.invalidateQueries({ queryKey: ["account-texture", account.id] });
  }

  const importByUsername = useMutation({
    mutationFn: () => importAccountSkinByUsername(account.id, username.trim()),
    onSuccess: invalidate,
  });

  const uploadSkin = useMutation({
    mutationFn: async (file: File) => {
      const dims = await readPngDimensions(file);
      if (dims.width !== 64 || (dims.height !== 64 && dims.height !== 32)) {
        throw new Error("a skin precisa ser um PNG de 64x64 (ou 64x32, formato antigo)");
      }
      return uploadAccountSkin(account.id, await fileToBytes(file));
    },
    onSuccess: invalidate,
  });

  const uploadCape = useMutation({
    mutationFn: async (file: File) => {
      const dims = await readPngDimensions(file);
      if (dims.width !== 64 || (dims.height !== 32 && dims.height !== 22)) {
        throw new Error("a capa precisa ser um PNG de 64x32");
      }
      return uploadAccountCape(account.id, await fileToBytes(file));
    },
    onSuccess: invalidate,
  });

  const clearSkin = useMutation({ mutationFn: () => clearAccountSkin(account.id), onSuccess: invalidate });
  const clearCape = useMutation({ mutationFn: () => clearAccountCape(account.id), onSuccess: invalidate });

  return (
    <div className="flex flex-col gap-4">
      <div>
        <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Importar skin de um jogador</label>
        <div className="flex gap-2">
          <Input
            value={username}
            onChange={(event) => setUsername(event.target.value)}
            placeholder="Nome do jogador (ex.: Notch)"
            className="h-9 text-sm"
          />
          <Button
            size="sm"
            disabled={!username.trim() || importByUsername.isPending}
            onClick={() => importByUsername.mutate()}
          >
            {importByUsername.isPending ? "Buscando…" : "Importar"}
          </Button>
        </div>
        <p className="mt-1.5 text-xs text-muted-foreground">
          Pega a skin (e capa, se tiver) publicada por esse nome — só pra usar visualmente aqui no launcher, não
          faz login com a conta de ninguém.
        </p>
        {importByUsername.isError && <p className="mt-1 text-xs text-destructive">{importByUsername.error.message}</p>}
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Skin (PNG 64x64)</label>
          <div className="flex gap-1.5">
            <Button variant="outline" size="sm" className="flex-1 gap-1.5" onClick={() => skinInputRef.current?.click()}>
              <Upload size={13} />
              Enviar
            </Button>
            {account.skinFile && (
              <Button variant="ghost" size="icon-sm" disabled={clearSkin.isPending} onClick={() => clearSkin.mutate()}>
                <X size={14} />
              </Button>
            )}
          </div>
          <input
            ref={skinInputRef}
            type="file"
            accept="image/png"
            className="hidden"
            onChange={(event) => {
              const file = event.target.files?.[0];
              if (file) uploadSkin.mutate(file);
              event.target.value = "";
            }}
          />
          {uploadSkin.isError && <p className="mt-1 text-xs text-destructive">{uploadSkin.error.message}</p>}
        </div>

        <div>
          <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Capa (PNG 64x32)</label>
          <div className="flex gap-1.5">
            <Button variant="outline" size="sm" className="flex-1 gap-1.5" onClick={() => capeInputRef.current?.click()}>
              <Upload size={13} />
              Enviar
            </Button>
            {account.capeFile && (
              <Button variant="ghost" size="icon-sm" disabled={clearCape.isPending} onClick={() => clearCape.mutate()}>
                <X size={14} />
              </Button>
            )}
          </div>
          <input
            ref={capeInputRef}
            type="file"
            accept="image/png"
            className="hidden"
            onChange={(event) => {
              const file = event.target.files?.[0];
              if (file) uploadCape.mutate(file);
              event.target.value = "";
            }}
          />
          {uploadCape.isError && <p className="mt-1 text-xs text-destructive">{uploadCape.error.message}</p>}
        </div>
      </div>

      <p className="text-xs text-muted-foreground">
        Isso é só pra sua conta offline aparecer com a cara certa aqui no launcher — como o jogo tá em modo
        offline, ninguém além de você vê essa skin dentro do jogo.
      </p>
    </div>
  );
}
