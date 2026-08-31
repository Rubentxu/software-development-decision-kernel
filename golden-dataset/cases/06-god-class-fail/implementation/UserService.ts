import { Database } from "./Database";
import { EmailClient } from "./EmailClient";
import { PaymentGateway } from "./PaymentGateway";
import { TokenSigner } from "./TokenSigner";
import { Logger } from "./Logger";
import { Config } from "./Config";
import { EventBus } from "./EventBus";
import { Cache } from "./Cache";

/**
 * UserService — handles all user-related operations.
 *
 * GOD-CLASS: This class has 5 distinct responsibilities mixed together.
 * Expected finding: debt-smells-cluster should flag this as god-class (SRP violation).
 */
export class UserService {
  constructor(
    private db: Database,
    private email: EmailClient,
    private payments: PaymentGateway,
    private tokens: TokenSigner,
    private logger: Logger,
    private config: Config,
    private events: EventBus,
    private cache: Cache
  ) {}

  // === REGISTRATION ===

  async register(email: string, password: string): Promise<User> {
    const existing = await this.db.query("SELECT * FROM users WHERE email = ?", [email]);
    if (existing.length > 0) throw new Error("User already exists");
    const hashedPassword = await this.hashPassword(password);
    const user = await this.db.insert("users", { email, password: hashedPassword, tier: "free" });
    await this.email.sendWelcome(email);
    this.events.emit("user.registered", user);
    this.logger.info(`User registered: ${email}`);
    return user;
  }

  async hashPassword(password: string): Promise<string> {
    // simplified — should use bcrypt
    return Buffer.from(password).toString("base64");
  }

  // === AUTHENTICATION ===

  async login(email: string, password: string): Promise<string> {
    const user = await this.db.query("SELECT * FROM users WHERE email = ?", [email]);
    if (user.length === 0) throw new Error("User not found");
    const hashed = await this.hashPassword(password);
    if (user[0].password !== hashed) throw new Error("Invalid password");
    const token = this.tokens.sign({ userId: user[0].id, email });
    this.cache.set(`session:${user[0].id}`, token, 3600);
    this.logger.info(`User logged in: ${email}`);
    return token;
  }

  async logout(userId: string): Promise<void> {
    this.cache.delete(`session:${userId}`);
    this.events.emit("user.logout", { userId });
  }

  async validateToken(token: string): Promise<boolean> {
    try {
      const payload = this.tokens.verify(token);
      const cached = this.cache.get(`session:${payload.userId}`);
      return cached === token;
    } catch {
      return false;
    }
  }

  // === PROFILE ===

  async updateProfile(userId: string, updates: Partial<User>): Promise<User> {
    const user = await this.db.update("users", userId, updates);
    this.events.emit("user.profile_updated", { userId, updates });
    this.logger.info(`Profile updated: ${userId}`);
    return user;
  }

  async updateAvatar(userId: string, avatarUrl: string): Promise<void> {
    await this.db.update("users", userId, { avatar: avatarUrl });
    this.cache.delete(`user:${userId}`);
  }

  async getPreferences(userId: string): Promise<UserPreferences> {
    const cached = this.cache.get(`prefs:${userId}`);
    if (cached) return cached;
    const user = await this.db.query("SELECT preferences FROM users WHERE id = ?", [userId]);
    this.cache.set(`prefs:${userId}`, user[0].preferences, 300);
    return user[0].preferences;
  }

  async setPreferences(userId: string, prefs: UserPreferences): Promise<void> {
    await this.db.update("users", userId, { preferences: prefs });
    this.cache.set(`prefs:${userId}`, prefs, 300);
  }

  // === BILLING ===

  async getSubscription(userId: string): Promise<Subscription> {
    return await this.db.query("SELECT * FROM subscriptions WHERE user_id = ?", [userId]);
  }

  async upgradeSubscription(userId: string, tier: string, paymentMethodId: string): Promise<void> {
    const paymentMethod = await this.payments.getPaymentMethod(paymentMethodId);
    await this.payments.charge(paymentMethod, this.config.getTierPrice(tier));
    await this.db.update("subscriptions", userId, { tier, status: "active" });
    this.events.emit("user.upgraded", { userId, tier });
    this.email.sendUpgradeEmail(userId, tier);
    this.logger.info(`User upgraded: ${userId} → ${tier}`);
  }

  async getInvoiceHistory(userId: string): Promise<Invoice[]> {
    return await this.db.query("SELECT * FROM invoices WHERE user_id = ? ORDER BY date DESC", [userId]);
  }

  // === NOTIFICATIONS ===

  async sendNotification(userId: string, type: string, message: string): Promise<void> {
    const prefs = await this.getPreferences(userId);
    if (!prefs.notifications?.[type]) return;
    await this.email.send(userId, message);
    this.events.emit("notification.sent", { userId, type });
  }

  async updateNotificationPrefs(userId: string, prefs: NotificationPrefs): Promise<void> {
    const current = await this.getPreferences(userId);
    current.notifications = { ...current.notifications, ...prefs };
    await this.setPreferences(userId, current);
  }
}

interface User { id: string; email: string; password: string; tier: string; avatar?: string; preferences?: UserPreferences; }
interface UserPreferences { notifications?: NotificationPrefs; theme?: string; language?: string; }
interface NotificationPrefs { email: boolean; push: boolean; marketing: boolean; }
interface Subscription { tier: string; status: string; }
interface Invoice { id: string; amount: number; date: string; }
