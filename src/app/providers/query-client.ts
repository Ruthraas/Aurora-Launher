import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";

// Sem isso, um erro de query/mutation não tratado localmente (nenhum
// `onError` no `useQuery`/`useMutation` do componente) só existia no
// estado `isError` — se ninguém checasse, sumia em silêncio. Isso não
// mostra nada pro usuário (cada tela já trata o próprio erro visível),
// só garante que fica no console pra depuração.
function logQueryError(error: unknown, context: string) {
  console.error(`[react-query] ${context} falhou:`, error);
}

export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error, query) => logQueryError(error, `query ${JSON.stringify(query.queryKey)}`),
  }),
  mutationCache: new MutationCache({
    onError: (error, _vars, _ctx, mutation) => logQueryError(error, `mutation ${mutation.options.mutationKey ?? "(sem key)"}`),
  }),
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
      // Sem isso (default 0) toda remontagem de componente refazia a
      // query do zero — caro pra queries que batem em API externa
      // (get_mod_compatibility, get_recommended_optimizations). Mutations
      // relevantes já chamam `invalidateQueries` explicitamente, então
      // isso não atrasa nada que precise ficar fresco na hora.
      staleTime: 30_000,
    },
  },
});
