import type { Instance } from "@/lib/tauri/commands/instances";
import { ContentBrowser } from "@/features/discover/ContentBrowser";

export function ModsTab({ instance }: { instance: Instance }) {
  return (
    <div className="h-full">
      <ContentBrowser scope={{ instanceId: instance.id }} />
    </div>
  );
}
