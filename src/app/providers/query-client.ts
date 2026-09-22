import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
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
