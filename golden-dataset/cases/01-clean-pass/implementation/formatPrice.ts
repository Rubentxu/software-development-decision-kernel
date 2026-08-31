/**
 * Formats a price given in cents to a display string.
 *
 * @param cents - the price in cents (e.g., 1099 = $10.99)
 * @param currency - ISO currency code (e.g., "USD", "EUR")
 * @returns formatted price string
 */
export function formatPrice(cents: number, currency: string): string {
  const amount = cents / 100;
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency,
  }).format(amount);
}
