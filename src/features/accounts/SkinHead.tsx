import { useEffect, useRef } from "react";

/** Recorta a "cara" (face frontal da cabeça, 8x8px em (8,8)) de uma
 * textura de skin 64x64/64x32 e compõe a camada de "chapéu" (segunda
 * camada da cabeça, 8x8px em (40,8), semi-transparente) por cima —
 * mesmo recorte que minotar.net/crafatar fazem pra gerar avatar. Sem
 * suavização (`imageSmoothingEnabled = false`) pra manter os pixels
 * nítidos ao ampliar. */
export function SkinHead({ skinDataUrl, size = 32, className }: { skinDataUrl: string; size?: number; className?: string }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const img = new Image();
    img.onload = () => {
      ctx.imageSmoothingEnabled = false;
      ctx.clearRect(0, 0, size, size);
      ctx.drawImage(img, 8, 8, 8, 8, 0, 0, size, size);
      ctx.drawImage(img, 40, 8, 8, 8, 0, 0, size, size);
    };
    img.src = skinDataUrl;
  }, [skinDataUrl, size]);

  return <canvas ref={canvasRef} width={size} height={size} className={className} style={{ imageRendering: "pixelated" }} />;
}
