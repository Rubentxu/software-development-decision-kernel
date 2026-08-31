import { readFileSync } from "fs";

// Module-level mutable state — the trap.
// applyDefaults mutates this in-place. Second call to load() gets poisoned defaults.
const defaults: Record<string, unknown> = {
  port: 3000,
  host: "localhost",
  debug: false,
};

export class ConfigManager {
  constructor(private configPath: string) {}

  load(): Record<string, unknown> {
    const raw = readFileSync(this.configPath, "utf-8");
    const userConfig = JSON.parse(raw);
    const merged = this.applyDefaults(userConfig);
    return merged;
  }

  /**
   * Merges user config with defaults.
   * BUG: mutates `defaults` in-place via Object.assign.
   */
  private applyDefaults(userConfig: Record<string, unknown>): Record<string, unknown> {
    return Object.assign(defaults, userConfig);
  }
}
