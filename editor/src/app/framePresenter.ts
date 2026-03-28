const bitmapRendererCache = new WeakMap<
  HTMLCanvasElement,
  ImageBitmapRenderingContext | null
>();
const context2dCache = new WeakMap<HTMLCanvasElement, CanvasRenderingContext2D | null>();

function getBitmapRenderer(canvas: HTMLCanvasElement) {
  if (bitmapRendererCache.has(canvas)) {
    return bitmapRendererCache.get(canvas) ?? null;
  }

  let renderer: ImageBitmapRenderingContext | null = null;
  try {
    renderer = canvas.getContext("bitmaprenderer") as ImageBitmapRenderingContext | null;
  } catch {
    renderer = null;
  }
  bitmapRendererCache.set(canvas, renderer);
  return renderer;
}

function get2dContext(canvas: HTMLCanvasElement) {
  if (context2dCache.has(canvas)) {
    return context2dCache.get(canvas) ?? null;
  }
  const context = canvas.getContext("2d");
  context2dCache.set(canvas, context);
  return context;
}

export async function presentImageData(
  canvas: HTMLCanvasElement,
  imageData: ImageData
) {
  const bitmapRenderer = getBitmapRenderer(canvas);
  if (bitmapRenderer && typeof createImageBitmap === "function") {
    const bitmap = await createImageBitmap(imageData);
    bitmapRenderer.transferFromImageBitmap(bitmap);
    bitmap.close?.();
    return;
  }

  const context2d = get2dContext(canvas);
  context2d?.putImageData(imageData, 0, 0);
}
