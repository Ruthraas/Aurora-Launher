import { useTranslation } from "react-i18next";
import { navItems, settingsNavItem } from "./nav-items";
import { SidebarLink } from "./SidebarLink";
import { SidebarBrand } from "./SidebarBrand";

/** <640px: vira barra fixa embaixo (sem a marca, cabe mais ícone).
 * ≥640px: volta a ser a coluna de ícones à esquerda. */
export function Sidebar() {
  const { t } = useTranslation();

  return (
    <nav
      className="fixed inset-x-0 bottom-0 z-20 flex h-14 w-full shrink-0 items-center justify-around border-t border-border bg-surface
        sm:static sm:h-full sm:w-16 sm:flex-col sm:items-center sm:justify-start sm:gap-1.5 sm:border-t-0 sm:border-r sm:py-3"
    >
      <div className="hidden sm:block">
        <SidebarBrand />
      </div>
      {navItems.map((item) => (
        <SidebarLink key={item.to} item={item} label={t(item.labelKey)} />
      ))}

      <div className="hidden sm:block sm:flex-1" />

      <SidebarLink item={settingsNavItem} label={t(settingsNavItem.labelKey)} />
    </nav>
  );
}
