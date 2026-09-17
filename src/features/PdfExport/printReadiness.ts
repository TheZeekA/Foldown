interface PrintableImage {
  complete: boolean;
  addEventListener(name: "load" | "error", listener: () => void): void;
  removeEventListener(name: "load" | "error", listener: () => void): void;
}

interface PrintResources {
  fonts?: { ready: PromiseLike<unknown> };
  images: Iterable<PrintableImage>;
}

function waitForImage(image: PrintableImage): Promise<void> {
  if (image.complete) return Promise.resolve();
  return new Promise((resolve) => {
    const settle = () => {
      image.removeEventListener("load", settle);
      image.removeEventListener("error", settle);
      resolve();
    };
    image.addEventListener("load", settle);
    image.addEventListener("error", settle);
    if (image.complete) settle();
  });
}

export async function waitForPrintResources(resources: PrintResources): Promise<void> {
  const fonts = resources.fonts?.ready ?? Promise.resolve();
  await Promise.all([fonts, ...Array.from(resources.images, waitForImage)]);
}

export function shouldPrintRequest(requestId: string, lastPrintedRequestId: string | null): boolean {
  return requestId !== lastPrintedRequestId;
}
