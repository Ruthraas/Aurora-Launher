import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Eye, EyeOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { getSettings, setCurseForgeApiKey } from "@/lib/tauri/commands/settings";

export function CurseForgeApiKeySetting() {
  const queryClient = useQueryClient();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: getSettings });
  const [value, setValue] = useState("");
  const [reveal, setReveal] = useState(false);
  const [savedFlash, setSavedFlash] = useState(false);

  useEffect(() => {
    if (settings?.curseforgeApiKey) setValue(settings.curseforgeApiKey);
  }, [settings?.curseforgeApiKey]);

  const mutation = useMutation({
    mutationFn: (apiKey: string) => setCurseForgeApiKey(apiKey.trim() || null),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
      setSavedFlash(true);
      setTimeout(() => setSavedFlash(false), 1500);
    },
  });

  return (
    <div className="flex flex-col gap-2">
      <p className="text-sm text-muted-foreground">
        Chave pessoal da API do CurseForge — obtida em{" "}
        <a
          href="https://console.curseforge.com/"
          target="_blank"
          rel="noreferrer"
          className="text-primary hover:underline"
        >
          console.curseforge.com
        </a>
        . Fica só nesta máquina, nunca é enviada a nenhum outro lugar além da própria API do CurseForge.
      </p>
      <div className="flex gap-2">
        <div className="relative flex-1">
          <Input
            type={reveal ? "text" : "password"}
            value={value}
            onChange={(event) => setValue(event.target.value)}
            placeholder="Cole a chave aqui"
            className="pr-9 font-mono text-xs"
          />
          <button
            type="button"
            onClick={() => setReveal((r) => !r)}
            className="absolute top-1/2 right-2.5 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          >
            {reveal ? <EyeOff size={14} /> : <Eye size={14} />}
          </button>
        </div>
        <Button size="sm" disabled={mutation.isPending} onClick={() => mutation.mutate(value)}>
          {savedFlash ? <Check size={14} /> : "Salvar"}
        </Button>
      </div>
      {mutation.isError && <p className="text-xs text-destructive">{mutation.error.message}</p>}
    </div>
  );
}
