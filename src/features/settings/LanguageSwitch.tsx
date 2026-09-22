import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

const languages = [
  { code: "pt-BR", label: "Português" },
  { code: "en", label: "English" },
];

export function LanguageSwitch() {
  const { i18n } = useTranslation();

  return (
    <div className="flex gap-2">
      {languages.map((lang) => (
        <Button
          key={lang.code}
          size="sm"
          variant={i18n.resolvedLanguage === lang.code ? "default" : "outline"}
          onClick={() => void i18n.changeLanguage(lang.code)}
        >
          {lang.label}
        </Button>
      ))}
    </div>
  );
}
