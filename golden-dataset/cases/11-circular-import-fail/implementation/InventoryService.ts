// InventoryService.ts
import { OrderService } from "./OrderService";

export class InventoryService {
  private orderService: OrderService;

  constructor() {
    // Circular dependency: OrderService needs InventoryService,
    // InventoryService needs OrderService
    this.orderService = new OrderService(this);
  }

  async reserveStock(productId: string, quantity: number): Promise<boolean> {
    const available = await this.checkStock(productId);
    return available >= quantity;
  }

  async checkStock(productId: string): Promise<number> {
    return 10; // simplified
  }

  async createBackOrder(productId: string, quantity: number): Promise<Order> {
    // This calls back into OrderService.createOrder — completing the cycle
    return await this.orderService.createOrder(productId, quantity);
  }
}

interface Order { productId: string; quantity: number; status: string; }
