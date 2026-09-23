import { useRef } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Loader2, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import { importInstanceFromBytes } from "@/lib/tauri/commands/instances";
import { instancesQueryKey } from "./use-instances";

async function fileToBytes(file: File): Promise<number[]> {
  const buffer = await file.arrayBuffer();
  return Array.from(new Uint8Array(buffer));
}

/** Importa um `.zip` exportado por outra instalação do Aurora Launcher
 *  (ver `InstanceCard`'s botão de exportar) — sem diálogo nativo de
 *  arquivo, usa `<input type="file">` normal e manda os bytes pro
 *  backend, mesmo padrão já usado pra upload de skin. */
export function ImportInstanceButton() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const inputRef = useRef<HTMLInputElement>(null);

  const importMutation = useMutation({
    mutationFn: (file: File) => fileToBytes(file).then(importInstanceFromBytes),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: instancesQueryKey }),
  });

  return (
    <>
      <input
        ref={inputRef}
        type="file"
        accept=".zip"
        className="hidden"
        onChange={(event) => {
          const file = event.target.files?.[0];
          if (file) importMutation.mutate(file);
          event.target.value = "";
        }}
      />
      <Button variant="outline" className="gap-1.5" disabled={importMutation.isPending} onClick={() => inputRef.current?.click()}>
        {importMutation.isPending ? <Loader2 size={14} className="animate-spin" /> : <Upload size={14} />}
        {t("instancesPage.import")}
      </Button>
      {importMutation.isError && <p className="mt-1 text-xs text-destructive">{importMutation.error.message}</p>}
    </>
  );
}
