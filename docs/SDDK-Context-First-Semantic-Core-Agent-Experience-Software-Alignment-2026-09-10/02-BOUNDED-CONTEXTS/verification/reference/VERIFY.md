# Verify — recent change alignment

Verify owns the recent delta.

1. compute source/fingerprint delta;
2. compute impact closure;
3. mark dependent knowledge possibly stale;
4. run cheap/base analysis;
5. use CodeIntelligence provider when available/valuable;
6. retrieve previous cards/assertions;
7. ask LLM only where semantic assessment is needed;
8. update affected knowledge/workbooks;
9. emit VerifyReceipt + deepening candidates.

The default goal is to **maintain synchronization**, not rediscover architecture.
