import { Order } from "./Order";
import { CustomerRepository } from "./CustomerRepository";

export class DiscountCalculator {
  constructor(private customers: CustomerRepository) {}

  async applyDiscount(order: Order): Promise<void> {
    // Feature envy: this method calls 4 methods on `order` and 0 on `this`
    const customerId = order.getCustomerId();
    const items = order.getItems();
    const total = order.getTotal();

    const customer = await this.customers.findById(customerId);
    const tier = customer?.loyaltyTier ?? "standard";

    let discount = 0;
    if (tier === "gold") discount = total * 0.10;
    else if (tier === "silver") discount = total * 0.05;

    if (items.length > 5) discount += total * 0.02; // bulk bonus

    order.setDiscount(discount);
  }
}
