// OrderService.ts
import { InventoryService } from "./InventoryService";

export class OrderService {
  constructor(private inventory: InventoryService) {}

  async createOrder(productId: string, quantity: number): Promise<Order> {
    const reserved = await this.inventory.reserveStock(productId, quantity);
    if (!reserved) {
      // Circular call: InventoryService.createBackOrder calls OrderService.createOrder
      return await this.inventory.createBackOrder(productId, quantity);
    }
    return { productId, quantity, status: "confirmed" };
  }
}

interface Order { productId: string; quantity: number; status: string; }
