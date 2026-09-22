import { Component, type ErrorInfo, type ReactNode } from "react";
import { TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";

interface State {
  error: Error | null;
}

/** Rede de segurança pra erro de render não tratado — sem isso, uma
 *  exceção em qualquer componente branqueava a tela inteira sem
 *  recuperação (React desmonta a árvore inteira quando ninguém
 *  captura). Fica só no topo do app, não em cada tela — se algo mais
 *  granular precisar de fallback próprio, cada feature cuida do seu
 *  caso de erro (ex.: `isError` do React Query), isso aqui é só o
 *  último recurso. */
export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary]", error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;

    return (
      <div className="flex h-screen flex-col items-center justify-center gap-3 bg-background p-6 text-center">
        <TriangleAlert size={32} className="text-destructive" />
        <p className="text-base font-semibold text-foreground">Algo deu errado</p>
        <p className="max-w-sm text-sm text-muted-foreground">
          A tela travou de um jeito inesperado. Recarregar deve resolver — se continuar acontecendo, é um bug de
          verdade.
        </p>
        <Button variant="outline" onClick={() => window.location.reload()}>
          Recarregar
        </Button>
      </div>
    );
  }
}
