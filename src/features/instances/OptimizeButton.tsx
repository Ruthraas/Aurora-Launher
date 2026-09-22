import { useEffect, useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, Loader2, Wand2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { optimizeInstance } from "@/lib/tauri/commands/optimization";
import { useJobProgressStore } from "@/features/discover/job-progress-store";
import { jobPhaseLabels } from "@/features/discover/job-phase-labels";

/** Botão "OTIMIZAR" da página de instância — varredura completa numa
 *  tacada só (mesmo backend do card "Otimizar desempenho" da Visão
 *  geral, só que sem precisar abrir o checklist: instala tudo do
 *  catálogo compatível com essa instância + aplica o `options.txt`).
 *  Fica no cabeçalho porque faz sentido em qualquer aba, não só na
 *  Visão geral. */
export function OptimizeButton({ instanceId }: { instanceId: string }) {
  const queryClient = useQueryClient();
  const [jobId, setJobId] = useState<string | null>(null);
  const [justOptimized, setJustOptimized] = useState(false);
  const job = useJobProgressStore((state) => (jobId ? state.jobs[jobId] : undefined));

  const optimize = useMutation({
    mutationFn: () => optimizeInstance(instanceId),
    onSuccess: (id) => setJobId(id),
  });

  const running = jobId && job && job.phase !== "finished" && job.phase !== "failed" && job.phase !== "cancelled";

  const handledJobId = useRef<string | null>(null);
  useEffect(() => {
    if (job?.phase !== "finished" || !jobId || handledJobId.current === jobId) return;
    handledJobId.current = jobId;
    void queryClient.invalidateQueries({ queryKey: ["optimizations", instanceId] });
    void queryClient.invalidateQueries({ queryKey: ["instance-mods", instanceId] });
    setJustOptimized(true);
    const timer = setTimeout(() => setJustOptimized(false), 2000);
    return () => clearTimeout(timer);
  }, [job?.phase, jobId, instanceId, queryClient]);

  return (
    <Button
      variant="outline"
      className="gap-1.5"
      disabled={optimize.isPending || Boolean(running)}
      onClick={() => optimize.mutate()}
      title="Varredura completa: instala tudo que for compatível para otimizar essa instância e ajusta o options.txt"
    >
      {justOptimized ? (
        <>
          <Check size={14} className="text-primary" />
          Otimizado
        </>
      ) : optimize.isPending || running ? (
        <>
          <Loader2 size={14} className="animate-spin" />
          {job ? jobPhaseLabels[job.phase] : "Preparando…"}
        </>
      ) : (
        <>
          <Wand2 size={14} />
          Otimizar
        </>
      )}
    </Button>
  );
}
