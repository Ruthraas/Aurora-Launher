# Aurora Launcher

Launcher de Minecraft Java (Windows-first), por **Aurora Technology Society (ATS)**.
Não afiliado, associado ou endossado pela Mojang Studios ou Microsoft.

**Stack**: Tauri 2 (Rust) + React + TypeScript + Vite + Tailwind v4 + shadcn/ui.
**Identifier**: `dev.ats.aurora-launcher`. **Package Rust**: `aurora-launcher` (lib `aurora_launcher_lib`).

Este README existe pra qualquer sessão (ou eu mesmo, depois) não precisar re-explorar o projeto do zero.
Mantenha atualizado a cada mudança relevante — é o que evita re-gastar tempo/tokens redescobrindo isso tudo.

## Como rodar

```powershell
npm install          # já instalado normalmente
npm run tauri dev     # sobe front (vite, porta 1420) + backend Rust, janela nativa
npm run build         # build de produção do front (tsc + vite)
cd src-tauri && cargo test    # 55 testes Rust, todos sem rede
cd src-tauri && cargo check   # checagem rápida
```

Se a porta 1420 já estiver em uso (processo `tauri dev` anterior não finalizado), o `tauri dev` novo falha —
mate o processo dono da porta antes (`Get-NetTCPConnection -LocalPort 1420` no PowerShell).

Dados do app em runtime: `%APPDATA%\dev.ats.aurora-launcher\`
- `accounts.json` — contas.
- `instances\<uuid>\instance.json` — cada instância (+ `logs/`, `crash-reports/`, `mods/`, etc. do próprio jogo).
- `cache\` — **cache global compartilhado**, nunca duplicado por instância: `libraries/`, `assets/`, `versions/`,
  `natives/<mc_version>/`, `runtimes/<java_component>/`, `neoforge/<neoforge_version>/` (instalador cacheado).

## Status por fase

- **Fase 1** (esqueleto) — feito. Tema, sidebar, i18n (PT-BR/EN), roteamento, cliente Tauri tipado.
- **Fase 2** (contas) — **offline completo**. Decisão de produto: Aurora Launcher é **offline-only**, sem login
  Microsoft — o variant `Account::Microsoft` foi removido do backend inteiro (não só "não implementado").
- **Fase 3** (vanilla) — feito. Manifesto, libraries+rules, assets, natives, Java automático, launch.
- **Fase 4** (instâncias + downloads) — feito. CRUD de instância, progresso em tempo real via eventos Tauri,
  retry/reinstalar.
- **Fase 5** (loaders) — **Fabric, NeoForge e Forge clássico prontos e testados de ponta a ponta** (instalar +
  jogar, confirmado pelo usuário — Forge validado em 1.20.2). Forge tem teto de versão (até 1.20.2, ver seção
  "Forge clássico"). **Quilt implementado mas AINDA NÃO testado de ponta a ponta** (código compila e os testes
  passam, ninguém instalou/jogou de verdade ainda) — reaproveita o mesmo pipeline do Fabric (profile idêntico,
  é um fork).
- **Fase 6** (Modrinth + CurseForge) — **busca, adicionar mod e baixar modpack funcionando nos dois**.
  - Tela "Descobrir" em lista (não grid de cards): busca com debounce, toggle Mods/Modpacks, toggle de fonte
    Ambos/Modrinth/CurseForge, filtro por loader.
  - **Adicionar mod a instância(s)**: modal calcula, por instância, se é compatível (versão do mod pra aquele
    loader+versão do MC), já instalado (via `mods.lock.json`) ou incompatível (com motivo) — resolve
    dependências obrigatórias e mostra "também será instalado". Seleção múltipla, pré-marca quando só uma
    instância é compatível.
  - **Baixar modpack**: cria a instância na hora (nome único automático, RAM sugerida por heurística) e roda um
    job em background — instala Minecraft+loader na versão exata do pack, baixa todos os mods com verificação
    de hash, extrai `overrides/` (proteção zip-slip), gera `mods.lock.json`. CurseForge com mods bloqueados pro
    autor (`downloadUrl: null`) não falha o job — marca a instância como "Incompleta" com a lista dos que
    faltam baixar na mão.
  - CurseForge exige chave pessoal (grátis, aprovação manual via console.curseforge.com) — configurável em
    Configurações → CurseForge, guardada em `settings.json`, **nunca no código-fonte**.
- **Fases 7-9** (otimizações, tela de detalhe de instância com aba "Mods"/logs/config) — não iniciadas.

## Arquitetura do backend (`src-tauri/src/core/`)

```
accounts/     Account (Microsoft | Offline), AccountStore (accounts.json)
minecraft/    manifest.rs (version_manifest_v2), version.rs (VersionDetails),
              rules.rs (avaliação allow/disallow), library.rs (resolve libraries/natives),
              arguments.rs (substituição ${...}), assets.rs, launch.rs (classpath/argv)
java/         runtime_manifest.rs + provision.rs — runtime Java gerenciado (nunca usa Java do sistema)
download/     file.rs (atômico+SHA1+retry), batch.rs (paralelo+progresso), archive.rs (zip utils),
              http.rs (client compartilhado, timeout 30s, retry 3x com backoff)
loaders/
  fabric.rs         meta.fabricmc.net — profile pronto (inheritsFrom + libs extras)
  installer.rs       pipeline compartilhado Forge/NeoForge (InstallProfile, processors, datamap)
  neoforge/          usa installer.rs + specifics do NeoForge (ver "NeoForge" abaixo)
  forge/             usa installer.rs + specifics do Forge clássico (ver "Forge clássico" abaixo)
  maven.rs           coordenada maven → path (compartilhado por todos os loaders)
instances/    Instance/LoaderKind/InstanceStore (instance.json), install.rs (orquestra tudo),
              launch.rs (funde vanilla+loader e spawna java), mods.rs (mods.lock.json por instância)
modrinth.rs   busca + versões + dependências de mods/modpacks (API pública, sem chave)
curseforge.rs busca + arquivos + dependências (API grátis mas com aprovação manual, exige chave — settings.rs)
mods_compat.rs cálculo de InstanceCompat (compatível/já instalado/incompatível) + instalação de 1 mod+deps
modpack.rs    parsing de .mrpack e manifest.json do CurseForge + instalação completa de um modpack
jobs.rs       registro de jobs em background (instalar mod / baixar modpack) — progresso via job://progress,
              cancelamento cooperativo (checado entre etapas, não aborta um download em voo)
settings.rs   settings.json (hoje só a chave do CurseForge) — nunca guardar segredo em outro lugar
```

Comandos Tauri ficam em `src-tauri/src/commands/*` — camada fina, chamam `core::*`.
Erros: `AppError` (Rust) → `AppErrorPayload { message }` (serializado pro front).

## Decisões e armadilhas já resolvidas (não redescobrir)

**UUID offline**: MD5 direto de `"OfflinePlayer:<nick>"`, SEM namespace — não é `Uuid::new_v3` de lib genérica.
Testado contra vetor real (`Notch` → `b50ad385-829d-3141-a216-7e7d7539ba7f`).

**Regras `rules[]` (allow/disallow)**: vale a ÚLTIMA regra que casar, não a primeira. `features` desconhecidas
(ex.: `is_quick_play_singleplayer`) precisam ser tratadas como **desativadas por padrão**, mesmo sem eu reconhecer
a chave — bug real que já causou crash ("Only one quick play option can be specified"). `os.versionRange` e
`os.arch` existem de verdade (build 26.3) e são tratados.

**Natives modernas (LWJGL 3.4+)**: uma biblioteca `:natives-<os>` precisa ir pro **classpath E ser extraída** —
LWJGL moderno se auto-extrai lendo o jar como recurso do classpath (via
`-Dorg.lwjgl.system.SharedLibraryExtractPath`), não só via `java.library.path`. Só extrair não bastava (causava
`UnsatisfiedLinkError: lwjgl.dll`).

**Download sem hash**: nem toda biblioteca publica SHA1 (ex.: o próprio jar do Fabric Loader) —
`download_file`/`DownloadTask.sha1` são `Option<String>`, pula verificação quando `None`.

**Fabric**: `meta.fabricmc.net/v2/versions/loader/<mc>/<loader>/profile/json` já devolve profile pronto
(`inheritsFrom` a versão vanilla). Bibliotecas do Fabric usam schema diferente (sem `downloads.artifact`, só
`name`+`url` base) — resolvido via `maven.rs`.

**Quilt**: fork do Fabric, `meta.quiltmc.org/v3/versions/loader/<mc>/<loader>/profile/json` devolve o profile
no MESMO formato byte-a-byte do Fabric (`inheritsFrom` + bibliotecas `name`+`url`, sem `downloads.artifact`) —
confirmado contra a API real. `core/loaders/quilt.rs` é essencialmente uma cópia adaptada de `fabric.rs` (URL
base + endpoint de versões diferentes, resto idêntico). Sem teto de versão conhecido, igual o Fabric.

**NeoForge**: o instalador (`maven.neoforged.net/releases/.../neoforge-<v>-installer.jar`) tem
`install_profile.json` com `processors[]` — ferramentas Java reais que rodam sequencialmente e bloqueantes
(diferente do lançamento do jogo, que é solto) pra gerar o jar "patchado". Pontos que já morderam:
- O jar patchado (`{PATCHED}`) só tem Minecraft+hooks — as classes do mod loader em si
  (`NeoForgeMod`, `ClientNeoForgeMod`) vêm de um jar **separado**, o `:universal` (também listado nas
  `libraries` do `install_profile.json`), que precisa entrar no classpath junto.
- A partir da Minecraft 1.21.7, o FancyModLoader exige o atributo de manifesto `Minecraft-Dists` no jar
  patchado — atributo que só o Gradle do NeoForge adiciona normalmente, não o instalador. Sem isso: "Fatal
  Startup Error" mesmo com o jar certo. `ensure_manifest_attribute()` (`core/download/archive.rs`) reescreve o
  manifesto depois dos processors rodarem. Duas armadilhas reais nessa função: (1) `add_directory` com nome já
  terminado em `/` duplicava a barra e corrompia zips grandes ("not a jar file") — corrigido tratando toda
  entrada via `start_file`; (2) o atributo precisa ir na seção **principal** do manifest (antes da primeira
  linha em branco), nunca colado no fim do arquivo — um manifest de jar real tem milhares de seções
  `Name:`/`Digest:` por entrada depois da principal, e um atributo solto no fim vira lixo de continuação da
  última entrada, não um atributo global (o FML simplesmente não via o `Minecraft-Dists`).
- Versão do NeoForge = versão do MC sem o `"1."` inicial + `.<build>[-beta]` (ex.: `1.21.11` → prefixo `21.11.`).

**Erro na UI**: toda falha de instalação fica em `instance.errorMessage` (persistido) e aparece de verdade na
tela — não só "falhou". Falha de processor do NeoForge inclui as últimas linhas do `stderr` na mensagem.

**Forge clássico (1.16+)**: mesmo instalador/processors do NeoForge (é o fork original) — reaproveita
`loaders/installer.rs`. Diferenças reais confirmadas baixando instaladores de verdade (1.16.5 e 1.20.1):
- Versão combinada `<mc_version>-<build>` (ex.: `1.20.1-47.2.0`), diferente do NeoForge que separa os dois.
  Lista de versões vem de `maven-metadata.xml` puro (sem API JSON dedicada como o NeoForge) — parseado por
  string simples (`parse_maven_versions`), sem trazer dependência de XML só pra isso.
- `data` do `install_profile.json` traz `client` E `server` em cada entrada (NeoForge só publica `client`) —
  ignoramos `server`, sem problema pro serde.
- Além do jar `PATCHED` + `:universal` (igual NeoForge), o Forge clássico precisa de um **terceiro** jar no
  classpath do lançamento: o `client-extra` (entrada `MC_EXTRA` do `data`, contém os assets do client.jar
  reempacotados) — confirmado pelo `-DignoreList=...,client-extra,...` que aparece nos jvm args do
  `version.json`. Sem ele, faltam texturas/idiomas embutidos no jar.
- `mainClass` muda por era mas isso já vem pronto no `version.json`, sem precisar branch por versão:
  `cpw.mods.modlauncher.Launcher` (1.16), `cpw.mods.bootstraplauncher.BootstrapLauncher` (1.20.1),
  `net.minecraftforge.bootstrap.BootstrapLauncher` (1.20.2). Como todo argumento/mainClass é lido do JSON
  (nunca hardcoded), o mesmo código cobre essas eras sem modificação.
- **NÃO tem** o requisito de manifesto `Minecraft-Dists` do NeoForge — isso é específico do FancyModLoader
  (NeoForge), o Forge clássico usa outro carregador (`fmlloader`) sem essa exigência.
- **Teto real: 1.20.2.** A partir da 1.20.4 (confirmado também em 1.21, 1.21.1 e builds mais novas) o instalador
  passa a publicar uma biblioteca `:shim` e o `mainClass` vira `net.minecraftforge.bootstrap.ForgeBootstrap` —
  uma arquitetura nova onde o `client.jar` vem num formato "bundler" com as próprias bibliotecas embutidas
  (processors `BUNDLER_EXTRACT`), sem `-cp`/`-p` nos jvm args. Não implementada ainda. **`MC_EXTRA` NÃO é um
  sinal confiável** de qual arquitetura é (testado: já sumia em 1.20.4 mesmo com a versão ainda usando launch
  clássico) — o sinal real e testado é a presença da biblioteca `:shim`, checado em `require_supported_architecture()`
  (`core/loaders/forge/install.rs`) ANTES de baixar qualquer coisa, pra não queimar banda numa instalação que
  vai falhar. Importante: essa mudança de arquitetura acontece por **build do Forge**, não por versão do
  Minecraft — builds diferentes da MESMA versão do MC podem estar em arquiteturas diferentes, então a faixa
  "1.16-1.20.2" do front-end (`loader-version-range.ts`) é só um filtro otimista, a checagem de verdade é
  sempre a do backend.
- **Confirmado de ponta a ponta pelo usuário em 1.20.2** (Forge 48.0.0) — instalou, abriu, Forge inicializou,
  som/texturas carregaram normal.
- **Argumento de processor com coordenada CRUA (sem `{CHAVE}`)**: bug real confirmado em 1.20.2 — o processor
  `MCP_DATA` recebe `--input [de.oceanlabs.mcp:mcp_config:<v>@zip]` direto no `args`, sem placeholder nenhum
  envolvendo (mesmo formato de uma entrada de `data`, só que cru). `substitute_processor_arg` só resolvia
  `{CHAVE}` vindo do datamap — sem tratar isso, a ferramenta recebia o argumento com colchete e tudo e quebrava
  com `"Input does not exist: [...]"`. Corrigido com `resolve_processor_arg()` (`core/loaders/installer.rs`):
  se o argumento INTEIRO bate com o formato `[coordenada]`/`'literal'`, resolve igual a uma entrada de `data`
  antes de cair na substituição de `{CHAVE}`.
- **`${classpath_separator}` no `-p` (module path) do BootstrapLauncher**: bug real confirmado em 1.20.2 — o
  jvm arg `-p` do Forge monta uma lista de jars separados por `${classpath_separator}` (placeholder Mojang
  `${...}`, não `{CHAVE}` de processor), que `LaunchVariables`/`substitute_placeholders`
  (`core/minecraft/arguments.rs`) não conhecia. Sem resolver, a JVM recebia o `${classpath_separator}` literal
  no meio do path e recusava com `InvalidPathException: Illegal char <:>`. Corrigido adicionando a chave
  `classpath_separator` (`;` no Windows, `:` no resto) no `lookup()`.
- **Java sem log nenhum quando crasha antes do Minecraft inicializar**: `launch()` (`core/instances/launch.rs`)
  jogava stdout/stderr do processo Java pro vazio (`Stdio::null()`) — um crash ANTES do log4j do jogo subir
  (classpath errado, módulo faltando) não deixava rastro nenhum, "não inicia" sem explicação. Corrigido: agora
  sempre grava em `<instância>/logs/launcher-stdout.log` e `launcher-stderr.log`, além do `logs/latest.log` que
  o próprio jogo cria quando chega a inicializar. Foi assim que os dois bugs acima foram encontrados.

## Redesign visual ("Aurora" — 2026-09-21, a partir de guia.txt + screens/)

Paleta/tipografia/logo/responsividade novos aplicados: tokens em `src/styles/globals.css` (fundo `#0d1117`,
accent teal `#2dd4bf`, sem gradiente chamativo fora das barras de progresso), fontes Sora (títulos)/DM Sans
(texto) via `@fontsource-variable`, logo cubo em `src/components/aurora-logo.tsx` (usado na sidebar, titlebar e
gerado pra ícone do app via `npm run tauri icon`). Sidebar vira barra inferior fixa abaixo de 640px
(`src/app/layout/Sidebar.tsx`).

**Simplificações conscientes** (por escopo/tempo, não por esquecimento):
- A tela "Instância e otimização" do mockup (visão geral/mods/config/logs por instância, sugestão de mods de
  otimização) **não existe** — não estava nas partes A/B/C descritas no texto do guia, só apareceu como uma
  captura extra. Fica pra depois se for pedida.
- Modais não viram bottom sheet full-screen no mobile — o Dialog do shadcn já limita a `calc(100%-2rem)` de
  largura, razoavelmente usável, mas não é o comportamento exato pedido.
- "Criar instância compatível" (dentro do modal de adicionar mod, quando nenhuma instância serve) abre o
  diálogo genérico de criar instância, não vem pré-preenchido com o loader/versão do mod — o usuário escolhe
  na mão.
- Cancelamento de job (`cancel_job`) existe no backend e é cooperativo, mas checado só entre arquivos/fases —
  não aborta um download já em andamento. O botão de cancelar não foi conectado na UI ainda (só o comando
  existe).
- `fetch_version_by_hash`/`fetch_mod_by_fingerprint` existem no backend (Modrinth e CurseForge) mas **não
  estão conectados a nada** — a detecção de mods colocados manualmente na pasta `mods/` (via hash) que o guia
  pede não foi implementada, só a infraestrutura pra isso ficou pronta.
- CurseForge: o shape de `dependencies[]` (`relationType`) é da documentação oficial, não foi confirmado contra
  um exemplo real com dependência não-vazia (testei vários mods, nenhum tinha pra a combinação testada).
- i18n: strings novas de B/C ficaram em PT-BR direto no componente, seguindo o padrão que o resto do app já
  usa pra texto de tela (só nav/home/instances/discover/accounts/settings passam por `t()`) — não criei chaves
  EN pra elas.
- Testes cobrem a lógica pura (parsing de .mrpack/manifest.json, loader de cada um, heurística de RAM,
  mods.lock.json, nome único de instância) — não há teste de integração ponta-a-ponta do job de instalação
  (precisaria de rede/arquivos grandes).

## Otimização + tela de detalhe da instância (2026-09-21)

Respondendo às perguntas que motivaram isso: **não**, os mods de otimização **não** entram sozinhos na criação
da instância — ficam numa lista curada, opt-in, na aba "Visão geral" da tela de detalhe (`/instances/:id`).
Motivo: instalar mod sem pedir é o tipo de coisa que quebra confiança do usuário num launcher; melhor mostrar
a lista com checkbox (a maioria já vem marcada, mas dá pra desmarcar) e um botão "Instalar N mod(s)" explícito
— o mesmo padrão do modal de adicionar mod (Parte B).

**Catálogo curado** (`core/optimization.rs`) — Fabric/Quilt e Forge/NeoForge têm ecossistemas DIFERENTES de mods
de otimização (confirmado contra a API real do Modrinth antes de montar a lista, não foi chute):
- **Fabric/Quilt**: Sodium (renderização), Lithium (lógica do jogo), FerriteCore (memória), Krypton (rede),
  ImmediatelyFast (render de itens/texto), EntityCulling (para de renderizar entidade fora de visão),
  ModernFix (tempo de carregamento + memória). Quilt reaproveita a mesma lista — na prática todo mod de
  otimização Fabric roda em Quilt via QFAPI, mesmo sem publicar uma versão com a tag "quilt" (por isso
  `mods_compat::compute_compatibility` também consulta o Modrinth como `"fabric"` pra instâncias Quilt, não
  `"quilt"` — sem esse fallback, toda instância Quilt via "Sem versão" pra praticamente qualquer mod).
- **NeoForge**: Sodium (builds nativas agora, não precisa de porta), Lithium, FerriteCore, ImmediatelyFast,
  EntityCulling, ModernFix.
- **Forge clássico**: **sem** Sodium (não existe build) — usa Embeddium (porta família-Sodium pra Forge/
  NeoForge) no lugar, + FerriteCore, EntityCulling, ModernFix.
- **Vanilla**: lista vazia (nada pra instalar, é vanilla mesmo).

**"Vanilla otimizado"** hoje significa só as **flags de JVM** (ver abaixo) — não mexe em `options.txt`
(distância de render, gráficos, etc.) ainda; isso ficaria fácil de adicionar depois no mesmo lugar
(`core/jvm_flags.rs` / aba Visão geral) se fizer sentido.

**Flags de JVM** (`core/jvm_flags.rs`): preset "G1GC otimizado" = as chamadas ["Aikar's flags"](https://docs.papermc.io/paper/aikars-flags)
— conjunto de flags de G1GC amplamente publicado e usado por servidores/launchers de Minecraft, não é invenção
nossa. Guardado por instância (`Instance::jvm_flag_preset`), aplicado em `launch.rs` antes de `-Xms`/`-Xmx`.
Editável na aba Visão geral (RAM + preset), salva na hora (`update_instance_settings`).

**Tela de detalhe da instância** (`/instances/:id`, `InstanceDetailPage.tsx`) — abre clicando no nome do card ou
no ícone de engrenagem:
- **Visão geral**: seção de otimização (acima) + slider de RAM + seletor de flags de JVM + "Abrir pasta".
- **Mods**: lista o que está instalado de verdade (lido do `mods.lock.json` via `list_instance_mods`, nunca
  inventado) com botão de remover (`remove_instance_mod` — apaga o arquivo E a entrada do lock, nunca só um);
  busca inline no Modrinth pra adicionar mais mods direto nessa instância (reaproveita `install_mod` passando
  só o id dela, sem precisar do modal de escolher instância da Parte B).
- **Configurações/Logs** (do mockup original) **não foram implementadas** — renomear/deletar já existem em
  Instâncias, e os logs já ficam em `<instância>/logs/*.log` (acessíveis via "Abrir pasta"); uma aba dedicada
  pra visualizar logs dentro do app fica pra depois se pedir.

**Resourcepacks/shaders/datapacks**: implementado. `core::mods_compat::ContentType` (Mod/ResourcePack/Shader/
Datapack — deliberadamente sem Modpack, que é um fluxo à parte, `core::modpack`) generaliza tudo que antes só
valia pra mod:
- Cada tipo tem sua pasta (`mods/`, `resourcepacks/`, `shaderpacks/`, `datapacks/`) — `ContentType::install_dir()`.
- Só `Mod` depende do loader da instância; resourcepack/shader/datapack rodam em QUALQUER instância, inclusive
  vanilla (confirmado: no Modrinth essas versões publicam `loaders: ["minecraft"]`, sem amarra nenhuma; no
  CurseForge não existe `modLoaderType` pra esses classIds — por isso `fetch_files_by_version` é uma variante
  sem esse filtro).
- CurseForge `classId` confirmado contra `GET /v1/categories?gameId=432` de verdade: 12=Resource Packs,
  6552=Shaders, 6945=Data Packs.
- **Datapack é uma simplificação consciente**: o formato certo do jogo é por-mundo (`saves/<mundo>/datapacks/`),
  não por-instância — instalamos na raiz da instância (`<instância>/datapacks/`) porque listar mundos e apontar
  pra um específico é uma frente de UI própria, não implementada ainda. Documentado aqui pra não ser esquecido.

**Descoberta/instalação unificada** (`ContentBrowser.tsx`, `features/discover/`): o antigo `DiscoverPage`
virou um componente parametrizável por `scope` — `"global"` (a tela Descobrir de verdade: Mods+Modpacks, todas
as instâncias como alvo, abre o modal de escolher instância pra mod ou o de baixar pra modpack) ou
`{ instanceId }` (dentro da aba Mods da tela de detalhe: Mods+Resource Packs+Shaders+Datapacks, instala DIRETO
nessa instância sem perguntar qual — já sabe qual é). Mesma UI rica nos dois (busca, toggle Modrinth/CurseForge/
Ambos, filtro de loader, seletor de layout Lista/Grade/Grade compacta) — pedido explícito do usuário
("tem que aparecer a lista igual ao CurseForge... e as opções de Modrinth/CurseForge") resolvido reaproveitando
o mesmo componente em vez de duas telas de busca divergentes.

## Auditoria completa do projeto → 17 issues fechadas (2026-09-22)

Pedido do usuário: analisar o projeto inteiro (front e back) e abrir issues
separadas por área no GitHub, depois resolver uma por uma via `gh` CLI. Um
agente fez a auditoria (sem tocar em código, só relatando); as 17 issues
(#1-#17) cobriram desde correções pontuais até features novas. Todas fechadas
nesta rodada, cada uma com commit próprio referenciando o número:

- **Validação/segurança**: nome de instância vazio/gigante agora é rejeitado;
  confirmação antes de excluir instância/conta; upload de skin/capa valida
  assinatura PNG + dimensão real do Minecraft; chave da API do CurseForge saiu
  do `settings.json` em texto puro e foi pro cofre de credenciais do SO (crate
  `keyring`), com migração automática de instalação existente.
- **Robustez**: timeout de download de arquivo grande separado do timeout de
  chamada de API pequena (30s matava download real de modpack lento); testes
  novos em `launch.rs` (ordem de montagem das flags de JVM) e `mods_compat.rs`
  (caminho "já instalado" — a mesma classe de bug que já apareceu 3x nesta
  sessão), ambas áreas sem nenhum teste até então.
- **Performance/UX**: checagem de compatibilidade de mod/otimização em
  paralelo (`futures::future::try_join_all`) em vez de uma instância/mod por
  vez em série; `staleTime` no `QueryClient`; anel de foco visível pra
  navegação por teclado; `ErrorBoundary` + `onError` global de query/mutation.
- **Infra**: CI real no GitHub Actions (`cargo test` + `clippy` + `tsc` +
  build do frontend em todo push/PR) e release automatizada numa tag `v*`
  (via `tauri-apps/tauri-action`) — as releases anteriores (v0.1.0/v0.2.0)
  foram buildadas na mão; a partir daqui o pipeline faz isso sozinho.
- **Features novas**: campo de flags de JVM customizadas (aplicadas por
  último, depois do preset); aba "Logs" (expõe os logs que o launcher já
  gravava, sem visualizador até então); export/import de instância inteira
  num `.zip` (config+mods+saves, sem o cache compartilhado — reinstala
  bibliotecas/assets automaticamente ao importar); aba "Mundos" (lista/abre
  pasta/apaga saves, com ícone e tamanho reais).
- **i18n**: sistema de tradução existia mas só 6 de 93 arquivos usavam de
  verdade — migradas as telas mais visíveis (instâncias, otimização, logs,
  mundos, diálogos de instalar mod/modpack), chaves sincronizadas 1:1 entre
  `pt-BR.json`/`en.json`. Não cobre 100% do app (Discover/Settings mais
  avançado ficou de fora) — decisão consciente de escopo, documentada na
  issue original.

## Otimização profissional + redesign da tela de instância (2026-09-21)

Pedido do usuário: melhorar o `options.txt` com mais técnicas reais de FPS, tirar
fog por padrão, criar um botão "OTIMIZAR" de varredura completa na tela de
instância, e deixar o design dessa tela mais parecido com o app oficial do
Modrinth. Além disso: corrigido um 404 real do Forge em modpacks (URL do
instalador sem a versão do Minecraft — ver seção anterior).

**`options.txt` — preset ampliado e verificado.** Toda chave nova foi conferida
contra `options.txt` reais publicados em modpacks (busca de código no GitHub),
nenhuma foi inventada. Adicionado `entityDistanceScaling:0.5` (corta a distância
de renderização de ENTIDADES — mobs, itens no chão — sem mexer no terreno; ganho
de FPS real em fazendas/servidores cheios). Removida a chave `clouds` que eu
tinha colocado numa rodada anterior — não existe no formato real, só `renderClouds`
existe; ficava lá sem efeito nenhum (o parser do Minecraft ignora chave
desconhecida silenciosamente, então não quebrava nada, mas não fazia nada
também).

**Sobre "tirar o fog": não é uma chave de config.** O `options.txt` vanilla nunca
teve um toggle de névoa — ela é renderizada com base na borda do `renderDistance`,
sem opção de desligar. O Sodium também **removeu** a opção de vídeo "Fog: Off"
das versões atuais (confirmado contra `sodium-options.json` reais: o schema
moderno só tem `use_fog_occlusion`, uma otimização interna, não um controle
visual). Por isso, adicionado ao catálogo curado (`core/optimization.rs`) o mod
dedicado **No Fog** (`QSzy55SB`, Modrinth, 1.15M+ downloads, Fabric/Forge/NeoForge/
Quilt) — como ele entra no catálogo, fica marcado por padrão no checklist de
otimização (a lógica existente já pré-seleciona tudo que é "compatível").

**Botão "Otimizar" — varredura completa.** Pedido explícito: um botão na tela de
instância que faz uma varredura geral e otimiza tudo numa tacada só, sem precisar
abrir o checklist manual. Novo comando `optimize_instance` (`commands/mods.rs`):
recalcula a compatibilidade do catálogo INTEIRO na hora (não confia em nada
computado antes — evita instalar algo que já não é mais compatível ou pular algo
que passou a ser), instala tudo que for compatível e ainda não estiver instalado,
e aplica o preset de `options.txt` no final — mesmo pipeline do fluxo manual
("Otimizar desempenho" na Visão geral), só que automático. O núcleo de instalação
foi extraído pra uma função compartilhada (`install_optimization_project_ids`)
entre os dois fluxos, pra não duplicar a lógica de "recalcula compatibilidade e
instala com dependências" duas vezes.

**Redesign da tela de instância (referência: app oficial do Modrinth).** Cabeçalho
próprio (`InstanceHeader.tsx`): ícone quadrado com a inicial da instância (sem
gradiente — a paleta do app reserva gradiente só pro logo da marca e barras de
progresso, ver comentário em `aurora-logo.tsx`), nome, e badges soltos de
versão/loader/RAM em vez de uma linha de texto corrida. Abas ganharam ícone ao
lado do texto. Lista de mods instalados (`InstalledContentList.tsx`) com linhas
maiores (ícone 40px), badges de versão/fonte/dependência em pill, e o botão de
remover só aparece no hover — menos ruído visual olhando a lista parada.

## Auditoria completa de qualidade (2026-09-21)

Pedido do usuário: revisão de código em todo o app, não só nos arquivos mexidos na sessão. Feita em duas
rodadas (um agente de auditoria em background, sem tocar em código, só relatando) + correção manual. Achados
reais, por ordem de gravidade:

1. **`ModLockEntry` sem `content_type`** — remover um resourcepack/shader/datapack instalado apagava só a
   entrada do `mods.lock.json`; o comando de remoção sempre olhava em `mods/`, então o arquivo de verdade
   (em `resourcepacks/`, `shaderpacks/` ou `datapacks/`) ficava órfão em disco pra sempre. `ContentType` foi
   movido de `mods_compat.rs` pra `core/instances/mods.rs` (é sobre onde o conteúdo mora na instância, faz
   mais sentido morar ali) e virou campo do `ModLockEntry`; `remove_instance_mod` agora usa
   `entry.content_type.install_dir()` em vez de `"mods"` fixo.
2. **Modpacks nunca entravam em `mods.lock.json`** — `install_modrinth_files`/`install_curseforge_files`
   baixavam tudo mas nunca registravam nada, quebrando a "única fonte de verdade" que o resto do app assume:
   aba Mods de uma instância vinda de modpack aparecia vazia, e o Discover oferecia "Adicionar" pra mods que
   já estavam lá. Corrigido nos dois formatos:
   - **CurseForge**: `manifest.json` já traz `projectID`/`fileID` reais — registro direto, sem gambiarra.
   - **Modrinth (`.mrpack`)**: o formato **não** traz `project_id`/`version_id` por arquivo (só `path`/
     `hashes`/`downloads`/`fileSize`/`env`, confirmado no formato real). Mas arquivos hospedados no CDN do
     Modrinth seguem um padrão de URL fixo e confirmado contra a API real:
     `cdn.modrinth.com/data/<project_id>/versions/<version_id>/<nome>` — `parse_modrinth_cdn_ids()` extrai os
     dois dali. Arquivo hospedado em outro lugar (o formato permite) simplesmente não entra no lock — continua
     instalado, só não fica rastreado por `(source, project_id)`. Limitação conhecida, documentada aqui.
3. **`AddModDialog` declarava sucesso antes do job terminar** — `installMod()` devolve o id do job assim que
   ele é ENFILEIRADO, não quando termina (mesma arquitetura fire-and-forget de todo o resto do app). O modal
   mostrava "Instalado." e invalidava a lista de compatibilidade nesse instante, enquanto o download de
   verdade ainda rodava — mesma classe de bug já corrigida em `OverviewTab`/`ContentBrowser` nesta sessão, só
   que esse componente nunca tinha sido migrado pro padrão certo (assinar `job://progress` via
   `useJobProgressStore`, só declarar sucesso em `phase === "finished"`). Corrigido: agora mostra barra de
   progresso de verdade durante a instalação.
4. **`loaderLabel` duplicado** — `DownloadModpackDialog.tsx` tinha sua própria cópia local do mapeamento
   loader→nome de exibição, já centralizado em `features/instances/loader-label.ts`. Duas cópias que podem
   divergir (ex.: mudar o nome de exibição do NeoForge e esquecer uma das duas). Consolidado: o util ganhou
   `loaderName()` (só o nome, sem versão) ao lado do `loaderLabel()` já existente (nome+versão), e o dialog
   passou a importar em vez de duplicar.

**Padrão geral encontrado 3 vezes nessa sessão** (vale a pena lembrar antes de adicionar qualquer coisa nova
que grave um id em `mods.lock.json`): NUNCA misturar convenções de identificador de fontes diferentes — slug
do Modrinth, id interno resolvido do Modrinth, id numérico do CurseForge. O que é gravado no lock tem que ser
EXATAMENTE o mesmo valor usado em toda consulta futura de "já instalado", sempre.

## Sistema de skin (2026-09-21)

Offline-only por decisão de produto, mas o launcher agora mostra a "cara" (avatar) de cada conta:
- **Importar por nome**: `core/skins.rs` — `api.mojang.com` resolve nome→UUID, `sessionserver.mojang.com`
  devolve o perfil com uma propriedade `textures` em base64 contendo a URL real da skin (e capa, se tiver).
  Mesma técnica que o SKlauncher usa — só "empresta" visualmente uma skin pública, não loga com a conta de
  ninguém. Confirmado contra a API real (perfil do "Notch").
- **Upload manual**: PNG de skin (64x64, ou 64x32 formato antigo) e capa (64x32) direto do usuário, validado no
  front (dimensões) antes de mandar pro backend.
- Guardado em `<app_data>/skins/<account_id>/{skin,cape}.png`; `Account::Offline` ganhou `skin_file`/
  `cape_file` (só o nome do arquivo, não o caminho inteiro).
- **`SkinHead.tsx`** recorta a cara (8x8 em (8,8) + camada de "chapéu" em (40,8)) via canvas, igual
  minotar/crafatar — usado no avatar da lista de contas.
- **Importante**: isso só muda o ícone dentro do launcher. Não muda a skin dentro do jogo — trocar a skin
  *no jogo* em modo offline exigiria um servidor de skin falso (tipo Ely.by) interceptando as texturas, que é
  uma frente completamente separada e não foi pedida/implementada.

## O que eu, especificamente, não devo tentar de novo sem avisar antes

- Login Microsoft: preciso de um Client ID do Azure AD que o usuário ainda não forneceu.
- Forge < 1.16: instalador GUI antigo que patcheava o jar vanilla direto, formato completamente diferente do
  moderno `install_profile.json` — não suportado, não tentar sem pesquisar do zero.
- Nada disso deve rodar downloads reais (Java runtime, instância) sem avisar — são dezenas/centenas de MB.
