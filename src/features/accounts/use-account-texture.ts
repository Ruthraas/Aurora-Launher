import { useQuery } from "@tanstack/react-query";
import { getAccountTexture } from "@/lib/tauri/commands/accounts";

export function useAccountTexture(accountId: string, texture: "skin" | "cape", enabled: boolean) {
  return useQuery({
    queryKey: ["account-texture", accountId, texture],
    queryFn: async () => {
      const base64 = await getAccountTexture(accountId, texture);
      return base64 ? `data:image/png;base64,${base64}` : null;
    },
    enabled,
  });
}
