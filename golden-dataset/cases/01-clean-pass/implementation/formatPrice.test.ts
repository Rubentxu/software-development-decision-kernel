import { describe, it, expect } from "vitest";
import { formatPrice } from "./formatPrice";

describe("formatPrice", () => {
  it("formats positive amount in USD", () => {
    expect(formatPrice(1099, "USD")).toBe("$10.99");
  });

  it("formats zero in EUR", () => {
    expect(formatPrice(0, "EUR")).toBe("€0.00");
  });

  it("formats negative amount in USD", () => {
    expect(formatPrice(-500, "USD")).toBe("-$5.00");
  });
});
