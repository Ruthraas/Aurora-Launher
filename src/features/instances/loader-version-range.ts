export type LoaderChoice = "vanilla" | "fabric" | "neoforge" | "forge" | "quilt";

/// Faixa de versão do Minecraft que cada loader suporta no backend —
/// mantém sincronizado com `core/loaders/*` (ver README, seção "Forge
/// clássico"/"NeoForge"). Forge tem teto porque a partir de uma versão
/// nova (~26.3) o instalador passou a usar uma arquitetura "shim"/
/// bundler que o Aurora Launcher ainda não implementa — o backend
/// recusa isso rápido, mas filtrar aqui evita o usuário nem tentar.
/// NeoForge nasceu como fork do Forge na 1.20.1, sem teto conhecido
/// até agora (confirmado funcionando na 26.3). Fabric não tem faixa
/// conhecida com problema, então não filtramos por versão.
export const LOADER_VERSION_RANGE: Record<LoaderChoice, { min?: string; max?: string }> = {
  vanilla: {},
  fabric: {},
  quilt: {},
  neoforge: { min: "1.20.1" },
  // teto real: 1.20.2 é a última versão confirmada sem a biblioteca
  // ":shim" (arquitetura nova do Forge, ainda não suportada) — testado
  // contra instaladores reais, 1.20.4 em diante já usa a nova.
  forge: { min: "1.16", max: "1.20.2" },
};

function parseVersionParts(id: string): number[] | null {
  if (!/^\d+(\.\d+)*$/.test(id)) return null;
  return id.split(".").map(Number);
}

function compareVersionParts(a: number[], b: number[]): number {
  const len = Math.max(a.length, b.length);
  for (let i = 0; i < len; i++) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

/// `true` quando `versionId` está dentro da faixa, OU quando não dá
/// pra classificar (snapshot como "24w14a", versões antigas tipo
/// "b1.7.3", esquemas futuros desconhecidos) — nesses casos preferimos
/// não esconder em vez de arriscar um filtro errado.
export function isVersionInLoaderRange(versionId: string, loaderKind: LoaderChoice): boolean {
  const { min, max } = LOADER_VERSION_RANGE[loaderKind];
  if (!min && !max) return true;

  const parts = parseVersionParts(versionId);
  if (!parts) return true;

  if (min && compareVersionParts(parts, parseVersionParts(min)!) < 0) return false;
  if (max && compareVersionParts(parts, parseVersionParts(max)!) > 0) return false;
  return true;
}
