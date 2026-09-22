import { useEffect, useRef } from "react";

const CANVAS_W = 16; // 4 (braço) + 8 (corpo) + 4 (braço)
const CANVAS_H = 32; // 8 (cabeça) + 12 (corpo) + 12 (pernas)

/** Renderização "de frente, parada" da skin inteira — cabeça, tronco,
 * braços e pernas, compostos num canvas só a partir das regiões fixas
 * do template de skin do Minecraft. Suporta os dois formatos de
 * textura (64x64 moderno, com braço/perna esquerdos separados; 64x32
 * antigo, espelhando o lado direito pro esquerdo). Não é o modelo 3D
 * do jogo — é um "boneco de papel" 2D, decorativo. */
export function SkinBodyRender({ skinDataUrl, height = 160, className }: { skinDataUrl: string; height?: number; className?: string }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const scale = height / CANVAS_H;
  const width = CANVAS_W * scale;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const img = new Image();
    img.onload = () => {
      ctx.imageSmoothingEnabled = false;
      ctx.clearRect(0, 0, width, height);
      const isModern = img.height >= 64;
      const draw = (sx: number, sy: number, sw: number, sh: number, dx: number, dy: number) =>
        ctx.drawImage(img, sx, sy, sw, sh, dx * scale, dy * scale, sw * scale, sh * scale);

      // pernas
      draw(4, 20, 4, 12, 4, 20); // direita
      if (isModern) {
        draw(20, 52, 4, 12, 8, 20); // perna esquerda de verdade (64x64)
      } else {
        // legado: espelha a perna direita pra esquerda
        ctx.save();
        ctx.translate(width, 0);
        ctx.scale(-1, 1);
        ctx.drawImage(img, 4, 20, 4, 12, (16 - 12) * scale, 20 * scale, 4 * scale, 12 * scale);
        ctx.restore();
      }

      // tronco
      draw(20, 20, 8, 12, 4, 8);

      // braços
      draw(44, 20, 4, 12, 0, 8); // direito
      if (isModern) {
        draw(36, 52, 4, 12, 12, 8); // esquerdo de verdade
      } else {
        ctx.save();
        ctx.translate(width, 0);
        ctx.scale(-1, 1);
        ctx.drawImage(img, 44, 20, 4, 12, 0, 8 * scale, 4 * scale, 12 * scale);
        ctx.restore();
      }

      // cabeça (+ camada "chapéu" por cima)
      draw(8, 8, 8, 8, 4, 0);
      draw(40, 8, 8, 8, 4, 0);

      // sobrecamada do tronco (jaqueta), se não for totalmente
      // transparente — desenha por cima, cobre pouco custo mesmo
      // quando a skin não tem essa camada (fica igual).
      draw(20, 36, 8, 12, 4, 8);
    };
    img.src = skinDataUrl;
  }, [skinDataUrl, scale, width, height]);

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      className={className}
      style={{ imageRendering: "pixelated", width, height }}
    />
  );
}
