export function isReady(status: string): boolean {
  return ["ready", "ok"].includes(status.toLowerCase());
}

export function testHappyPath(): void {
  if (!isReady("ok")) throw new Error("expected ok");
}
