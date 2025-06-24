# HoldReason Encoding Analysis

Based on my investigation of the Tanssi codebase, here's how the `HoldReason::PooledStake` is encoded:

## Key Findings:

1. **HoldReason Definition**:
   - In `pallets/pooled-staking/src/lib.rs`, the HoldReason is defined as:
   ```rust
   #[pallet::composite_enum]
   pub enum HoldReason {
       PooledStake,
   }
   ```

2. **Usage in Code**:
   - When creating holds, the code uses: `&HoldReason::PooledStake.into()`
   - This converts the pallet-specific `HoldReason` into the runtime's `RuntimeHoldReason`

3. **Runtime Composition**:
   - The `construct_runtime!` macro in the runtime combines all pallet HoldReasons into a single `RuntimeHoldReason` enum
   - Each pallet's HoldReason becomes a variant in the RuntimeHoldReason enum

4. **Actual Encoding**:
   - When queried from `balances.holds()`, the hold reason appears as a variant of `RuntimeHoldReason`
   - The format would be: `RuntimeHoldReason::PooledStaking(pallet_pooled_staking::HoldReason::PooledStake)`
   - In the runtime, this is typically represented as an object with the pallet name and the specific hold reason

5. **Example from Tests**:
   - In integration tests, holds are checked using:
   ```rust
   Balances::balance_on_hold(&pallet_registrar::HoldReason::RegistrarDeposit.into(), &account)
   ```
   - The mock tests use:
   ```rust
   pub fn balance_hold(who: &AccountId) -> Balance {
       Balances::balance_on_hold(&crate::HoldReason::PooledStake.into(), who)
   }
   ```

## Expected Format in Runtime Query:

When querying `balances.holds()` for an account with PooledStake holds, the hold reason in the returned data would likely be encoded as:
- A variant identifier for the pallet (e.g., "PooledStaking" or a numeric index based on the pallet's position in construct_runtime!)
- The specific hold reason within that pallet (e.g., "PooledStake")

The exact format depends on how the runtime serializes the enum, but it would typically be something like:
```json
{
  "id": {
    "PooledStaking": "PooledStake"
  },
  "amount": "..."
}
```

or with numeric encoding:
```json
{
  "id": [34, 0],  // 34 is the pallet index for PooledStaking in construct_runtime!, 0 is the variant index for PooledStake
  "amount": "..."
}
```