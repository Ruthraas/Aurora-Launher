import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Plus, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Slider } from "@/components/ui/slider";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Chip } from "@/components/chip";
import { cn } from "@/lib/utils";
import { useVersionManifest } from "./use-version-manifest";
import { useFabricLoaderVersions } from "./use-fabric-loader-versions";
import { useNeoForgeVersions } from "./use-neoforge-versions";
import { useForgeVersions } from "./use-forge-versions";
import { useQuiltLoaderVersions } from "./use-quilt-loader-versions";
import { useCreateInstance } from "./use-instance-mutations";
import { isVersionInLoaderRange, LOADER_VERSION_RANGE, type LoaderChoice } from "./loader-version-range";

const RAM_MIN_GB = 1;
const RAM_MAX_GB = 16;
const DEFAULT_RAM_GB = 4;
const MAX_VISIBLE_VERSIONS = 100;

export function CreateInstanceDialog() {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [mcVersion, setMcVersion] = useState("");
  const [query, setQuery] = useState("");
  const [showAllTypes, setShowAllTypes] = useState(false);
  const [loaderKind, setLoaderKind] = useState<LoaderChoice>("vanilla");
  const [loaderVersion, setLoaderVersion] = useState("");
  const [ramGb, setRamGb] = useState(DEFAULT_RAM_GB);

  const { data: manifest, isLoading: loadingVersions } = useVersionManifest();
  const { data: fabricLoaders, isLoading: loadingFabricLoaders } = useFabricLoaderVersions(
    mcVersion,
    loaderKind === "fabric",
  );
  const { data: neoforgeVersions, isLoading: loadingNeoForgeVersions } = useNeoForgeVersions(
    mcVersion,
    loaderKind === "neoforge",
  );
  const { data: forgeVersions, isLoading: loadingForgeVersions } = useForgeVersions(
    mcVersion,
    loaderKind === "forge",
  );
  const { data: quiltLoaders, isLoading: loadingQuiltLoaders } = useQuiltLoaderVersions(
    mcVersion,
    loaderKind === "quilt",
  );
  const createInstance = useCreateInstance();

  const versions = useMemo(() => {
    if (!manifest) return [];
    const q = query.trim().toLowerCase();
    return manifest.versions
      .filter((version) => showAllTypes || version.type === "release")
      .filter((version) => version.id.toLowerCase().includes(q))
      .filter((version) => isVersionInLoaderRange(version.id, loaderKind))
      .slice(0, MAX_VISIBLE_VERSIONS);
  }, [manifest, showAllTypes, query, loaderKind]);

  // ao trocar de versão/loader, o loader version escolhido antes deixa de valer
  useEffect(() => {
    setLoaderVersion("");
  }, [mcVersion, loaderKind]);

  // trocar de loader pode deixar a versão do Minecraft já escolhida
  // fora da faixa suportada (ex.: tava em Forge numa versão recente e
  // foi pra NeoForge) — em vez de deixar uma combinação inválida
  // escondida, já limpa pra forçar escolher de novo dentro da lista
  // filtrada.
  useEffect(() => {
    if (mcVersion && !isVersionInLoaderRange(mcVersion, loaderKind)) {
      setMcVersion("");
    }
  }, [loaderKind, mcVersion]);

  // assim que a lista de versões do loader carrega, já cravamos a mais
  // recente (topo da lista — todas as APIs devolvem mais nova primeiro)
  // em vez de obrigar escolha manual toda vez — quem quiser outra ainda
  // pode trocar no seletor.
  useEffect(() => {
    if (loaderKind === "fabric" && !loaderVersion && fabricLoaders && fabricLoaders.length > 0) {
      setLoaderVersion(fabricLoaders[0].loader.version);
    }
  }, [loaderKind, loaderVersion, fabricLoaders]);

  useEffect(() => {
    if (loaderKind === "neoforge" && !loaderVersion && neoforgeVersions && neoforgeVersions.length > 0) {
      setLoaderVersion(neoforgeVersions[0]);
    }
  }, [loaderKind, loaderVersion, neoforgeVersions]);

  useEffect(() => {
    if (loaderKind === "forge" && !loaderVersion && forgeVersions && forgeVersions.length > 0) {
      setLoaderVersion(forgeVersions[0]);
    }
  }, [loaderKind, loaderVersion, forgeVersions]);

  useEffect(() => {
    if (loaderKind === "quilt" && !loaderVersion && quiltLoaders && quiltLoaders.length > 0) {
      setLoaderVersion(quiltLoaders[0].loader.version);
    }
  }, [loaderKind, loaderVersion, quiltLoaders]);

  function reset() {
    setName("");
    setMcVersion("");
    setQuery("");
    setLoaderKind("vanilla");
    setLoaderVersion("");
    setRamGb(DEFAULT_RAM_GB);
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    createInstance.mutate(
      {
        name: name.trim(),
        mcVersion,
        loaderKind,
        loaderVersion: loaderKind !== "vanilla" ? loaderVersion : undefined,
        ramMinMb: 1024,
        ramMaxMb: ramGb * 1024,
      },
      {
        onSuccess: () => {
          setOpen(false);
          reset();
        },
      },
    );
  }

  const needsLoaderVersion = loaderKind !== "vanilla";
  const canSubmit = name.trim().length > 0 && mcVersion.length > 0 && (!needsLoaderVersion || loaderVersion.length > 0);

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) reset();
      }}
    >
      <DialogTrigger asChild>
        <Button size="sm" className="gap-1.5">
          <Plus size={14} />
          Nova
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Nova instância</DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="flex flex-col gap-5">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-muted-foreground" htmlFor="instance-name">
              Nome
            </label>
            <Input
              id="instance-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Meu Survival"
              autoFocus
            />
          </div>

          <div>
            <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Loader</label>
            <div className="flex gap-2">
              <Chip active={loaderKind === "vanilla"} onClick={() => setLoaderKind("vanilla")}>
                Vanilla
              </Chip>
              <Chip active={loaderKind === "fabric"} onClick={() => setLoaderKind("fabric")}>
                Fabric
              </Chip>
              <Chip active={loaderKind === "neoforge"} onClick={() => setLoaderKind("neoforge")}>
                NeoForge
              </Chip>
              <Chip active={loaderKind === "forge"} onClick={() => setLoaderKind("forge")}>
                Forge
              </Chip>
              <Chip active={loaderKind === "quilt"} onClick={() => setLoaderKind("quilt")}>
                Quilt
              </Chip>
            </div>
            {(LOADER_VERSION_RANGE[loaderKind].min || LOADER_VERSION_RANGE[loaderKind].max) && (
              <p className="mt-1.5 text-xs text-muted-foreground">
                Só mostrando versões do Minecraft suportadas por esse loader
                {LOADER_VERSION_RANGE[loaderKind].min && ` (${LOADER_VERSION_RANGE[loaderKind].min}+`}
                {LOADER_VERSION_RANGE[loaderKind].max && ` até ${LOADER_VERSION_RANGE[loaderKind].max}`}
                {LOADER_VERSION_RANGE[loaderKind].min && ")"}.
              </p>
            )}
          </div>

          <div>
            <div className="mb-1.5 flex items-center justify-between">
              <label className="text-xs font-medium text-muted-foreground">Versão do Minecraft</label>
              {mcVersion && <span className="text-xs font-medium text-primary">{mcVersion}</span>}
            </div>

            <div className="relative mb-2">
              <Search
                className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted-foreground"
                size={14}
              />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Buscar versão…"
                className="h-8 pl-8 text-sm"
              />
            </div>

            <div className="mb-2 flex gap-2">
              <Chip active={!showAllTypes} onClick={() => setShowAllTypes(false)}>
                Releases
              </Chip>
              <Chip active={showAllTypes} onClick={() => setShowAllTypes(true)}>
                Todas
              </Chip>
            </div>

            <div className="scroll-thin max-h-40 overflow-y-auto rounded-lg border border-border">
              {loadingVersions ? (
                <p className="p-3 text-xs text-muted-foreground">Carregando…</p>
              ) : versions.length === 0 ? (
                <p className="p-3 text-xs text-muted-foreground">Nenhuma versão encontrada.</p>
              ) : (
                versions.map((version) => (
                  <button
                    key={version.id}
                    type="button"
                    onClick={() => setMcVersion(version.id)}
                    className={cn(
                      "flex w-full items-center justify-between px-3 py-1.5 text-left text-sm transition-colors",
                      mcVersion === version.id
                        ? "bg-primary text-primary-foreground"
                        : "text-foreground hover:bg-white/5",
                    )}
                  >
                    <span>{version.id}</span>
                    {version.type !== "release" && (
                      <span className="text-[0.65rem] tracking-wide uppercase opacity-70">{version.type}</span>
                    )}
                  </button>
                ))
              )}
            </div>
          </div>

          {needsLoaderVersion && mcVersion && loaderKind === "fabric" && (
            <div>
              <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Versão do Fabric Loader</label>
              <Select value={loaderVersion} onValueChange={setLoaderVersion} disabled={loadingFabricLoaders}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder={loadingFabricLoaders ? "Carregando…" : "Escolha uma versão"} />
                </SelectTrigger>
                <SelectContent>
                  {fabricLoaders?.map((entry) => (
                    <SelectItem key={entry.loader.version} value={entry.loader.version}>
                      {entry.loader.version}
                      {entry.loader.stable ? "" : " (instável)"}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {!loadingFabricLoaders && fabricLoaders?.length === 0 && (
                <p className="mt-1.5 text-xs text-muted-foreground">
                  Sem build de Fabric Loader pra essa versão do Minecraft.
                </p>
              )}
            </div>
          )}

          {needsLoaderVersion && mcVersion && loaderKind === "neoforge" && (
            <div>
              <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Versão do NeoForge</label>
              <Select value={loaderVersion} onValueChange={setLoaderVersion} disabled={loadingNeoForgeVersions}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder={loadingNeoForgeVersions ? "Carregando…" : "Escolha uma versão"} />
                </SelectTrigger>
                <SelectContent>
                  {neoforgeVersions?.map((version) => (
                    <SelectItem key={version} value={version}>
                      {version}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {!loadingNeoForgeVersions && neoforgeVersions?.length === 0 && (
                <p className="mt-1.5 text-xs text-muted-foreground">
                  Sem build de NeoForge pra essa versão do Minecraft.
                </p>
              )}
            </div>
          )}

          {needsLoaderVersion && mcVersion && loaderKind === "forge" && (
            <div>
              <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Versão do Forge</label>
              <Select value={loaderVersion} onValueChange={setLoaderVersion} disabled={loadingForgeVersions}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder={loadingForgeVersions ? "Carregando…" : "Escolha uma versão"} />
                </SelectTrigger>
                <SelectContent>
                  {forgeVersions?.map((version) => (
                    <SelectItem key={version} value={version}>
                      {version}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {!loadingForgeVersions && forgeVersions?.length === 0 && (
                <p className="mt-1.5 text-xs text-muted-foreground">
                  Sem build de Forge pra essa versão do Minecraft.
                </p>
              )}
            </div>
          )}

          {needsLoaderVersion && mcVersion && loaderKind === "quilt" && (
            <div>
              <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Versão do Quilt Loader</label>
              <Select value={loaderVersion} onValueChange={setLoaderVersion} disabled={loadingQuiltLoaders}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder={loadingQuiltLoaders ? "Carregando…" : "Escolha uma versão"} />
                </SelectTrigger>
                <SelectContent>
                  {quiltLoaders?.map((entry) => (
                    <SelectItem key={entry.loader.version} value={entry.loader.version}>
                      {entry.loader.version}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {!loadingQuiltLoaders && quiltLoaders?.length === 0 && (
                <p className="mt-1.5 text-xs text-muted-foreground">
                  Sem build de Quilt Loader pra essa versão do Minecraft.
                </p>
              )}
            </div>
          )}

          <div>
            <div className="mb-1.5 flex items-center justify-between">
              <label className="text-xs font-medium text-muted-foreground">Memória</label>
              <span className="text-sm font-medium text-primary">{ramGb} GB</span>
            </div>
            <Slider
              min={RAM_MIN_GB}
              max={RAM_MAX_GB}
              step={1}
              value={[ramGb]}
              onValueChange={([value]) => setRamGb(value)}
            />
          </div>

          {createInstance.isError && <p className="text-xs text-destructive">{createInstance.error.message}</p>}

          <DialogFooter>
            <Button type="submit" disabled={createInstance.isPending || !canSubmit}>
              {createInstance.isPending ? "Criando…" : "Criar instância"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
